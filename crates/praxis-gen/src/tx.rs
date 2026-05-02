//! `TxComposer` — assembles a signed `Transaction` from an instruction + accounts + args.
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey::Pubkey,
    signer::Signer,
    transaction::Transaction,
};

use praxis_core::NormalInstruction;

use crate::AccountSet;

/// Builds `Transaction` values from fuzzer-generated account sets.
pub struct TxComposer;

impl TxComposer {
    /// Compose a transaction.
    ///
    /// `args_bytes` is a slice of raw-encoded argument values in instruction
    /// order. They are concatenated after the 8-byte discriminator to form
    /// the instruction data.  Pass `&[]` when there are no arguments.
    pub fn compose(
        program_id: Pubkey,
        ix_def: &NormalInstruction,
        accounts: &AccountSet,
        args_bytes: &[Vec<u8>],
        blockhash: Hash,
    ) -> Transaction {
        // Build the account-metas in slot order, consulting the IDL for flags.
        let account_metas: Vec<AccountMeta> = accounts
            .slots
            .iter()
            .map(|(name, entry)| {
                let meta = ix_def.accounts.iter().find(|a| a.name == *name);
                let is_signer = meta.map(|m| m.signer).unwrap_or(false)
                    && entry.keypair.is_some();
                let is_writable = meta.map(|m| m.writable).unwrap_or(false);
                if is_writable {
                    AccountMeta::new(entry.pubkey, is_signer)
                } else {
                    AccountMeta::new_readonly(entry.pubkey, is_signer)
                }
            })
            .collect();

        // Instruction data = discriminator || args...
        let mut data = ix_def.discriminator.clone();
        for arg in args_bytes {
            data.extend_from_slice(arg);
        }

        let instruction = Instruction {
            program_id,
            accounts: account_metas,
            data,
        };

        // Fee payer: first writable signer, or first signer, or any signer.
        let fee_payer = accounts.fee_payer();

        // Collect all signers (Arc<Keypair>) in a stable order.
        let keypairs = accounts.all_keypairs();
        let signers: Vec<&dyn Signer> = keypairs.iter().map(|k| k.as_ref() as &dyn Signer).collect();

        if signers.is_empty() {
            // No signers — build an unsigned transaction (will fail on-chain but
            // is useful for testing error paths).
            Transaction::new_unsigned(Message::new(&[instruction], fee_payer.as_ref()))
        } else {
            Transaction::new(&signers, Message::new(&[instruction], fee_payer.as_ref()), blockhash)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use praxis_core::{AccountConstraint, IxAccountMeta, NormalInstruction};
    use crate::AccountSpawner;
    use solana_sdk::hash::Hash;

    fn make_ix() -> NormalInstruction {
        NormalInstruction {
            name: "transfer".into(),
            discriminator: vec![163, 52, 200, 231, 140, 3, 69, 186],
            accounts: vec![
                IxAccountMeta {
                    name: "from".into(),
                    writable: true,
                    signer: true,
                    optional: false,
                    constraint: Some(AccountConstraint::Signer),
                },
                IxAccountMeta {
                    name: "to".into(),
                    writable: true,
                    signer: false,
                    optional: false,
                    constraint: None,
                },
            ],
            args: vec![],
        }
    }

    #[test]
    fn compose_builds_transaction_with_correct_program() {
        let program_id = Pubkey::new_unique();
        let ix = make_ix();
        let accounts = AccountSpawner::spawn(&ix, program_id, 11);
        let blockhash = Hash::new_unique();
        let tx = TxComposer::compose(program_id, &ix, &accounts, &[], blockhash);

        assert_eq!(tx.message.instructions.len(), 1);
        let compiled_ix = &tx.message.instructions[0];
        // Program ID is in the accounts list at the index given by program_id_index
        let prog_idx = compiled_ix.program_id_index as usize;
        let prog_key = tx.message.account_keys[prog_idx];
        assert_eq!(prog_key, program_id);
    }

    #[test]
    fn compose_includes_discriminator_in_data() {
        let program_id = Pubkey::new_unique();
        let ix = make_ix();
        let accounts = AccountSpawner::spawn(&ix, program_id, 12);
        let blockhash = Hash::new_unique();
        let tx = TxComposer::compose(
            program_id,
            &ix,
            &accounts,
            &[vec![1, 0, 0, 0, 0, 0, 0, 0]], // u64 arg = 1
            blockhash,
        );

        let data = &tx.message.instructions[0].data;
        // First 8 bytes = discriminator
        assert_eq!(&data[..8], &[163, 52, 200, 231, 140, 3, 69, 186]);
        // Arg bytes follow
        assert_eq!(&data[8..], &[1, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn compose_correct_account_count() {
        let program_id = Pubkey::new_unique();
        let ix = make_ix();
        let accounts = AccountSpawner::spawn(&ix, program_id, 13);
        let blockhash = Hash::new_unique();
        let tx = TxComposer::compose(program_id, &ix, &accounts, &[], blockhash);

        // 2 instruction accounts + program_id itself in the message account keys
        assert!(tx.message.account_keys.len() >= 2);
    }
}
