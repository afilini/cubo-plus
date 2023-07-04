mod utils;

use utils::*;

const BLOCK: &'static str = include_str!("../block.hex");

trait Parse: Sized {
    fn parse(bytes: &[u8]) -> Result<(Self, &[u8]), Error>;
}

#[derive(Debug)]
struct VarInt(u64);

impl Parse for VarInt {
    fn parse(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (val, remainder) = match bytes[0] {
            ..=0xFC => (bytes[0] as u64, &bytes[1..]),
            0xFD => (u16::from_le_bytes(bytes[1..3].try_into()?) as u64, &bytes[3..]),
            0xFE => (u32::from_le_bytes(bytes[1..5].try_into()?) as u64, &bytes[5..]),
            0xFF => (u64::from_le_bytes(bytes[1..9].try_into()?), &bytes[9..]),
        };

        Ok((VarInt(val), remainder))
    }
}

impl Parse for i32 {
    fn parse(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let val = i32::from_le_bytes(bytes[0..4].try_into()?);
        Ok((val, &bytes[4..]))
    }
}
impl Parse for u32 {
    fn parse(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let val = u32::from_le_bytes(bytes[0..4].try_into()?);
        Ok((val, &bytes[4..]))
    }
}
impl Parse for u8 {
    fn parse(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let val = bytes[0];
        Ok((val, &bytes[1..]))
    }
}
impl Parse for u64 {
    fn parse(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let val = u64::from_le_bytes(bytes[0..8].try_into()?);
        Ok((val, &bytes[8..]))
    }
}

impl Parse for [u8; 32] {
    fn parse(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let val = bytes[..32].try_into()?;
        Ok((val, &bytes[32..]))
    }
}

#[derive(Debug)]
struct BlockHeader {
    version: i32,
    prev_block: [u8; 32],
    merkle_root: [u8; 32],
    timestamp: u32,
    bits: u32,
    nonce: u32,
}

impl Parse for BlockHeader {
    fn parse(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (version, bytes) = Parse::parse(bytes)?;
        let (prev_block, bytes) = Parse::parse(bytes)?;
        let (merkle_root, bytes) = Parse::parse(bytes)?;
        let (timestamp, bytes) = Parse::parse(bytes)?;
        let (bits, bytes) = Parse::parse(bytes)?;
        let (nonce, bytes) = Parse::parse(bytes)?;

        let header = BlockHeader {
            version, prev_block, merkle_root, timestamp, bits, nonce,
        };

        Ok((header, bytes))
    }
}

#[derive(Debug)]
struct Block {
    header: BlockHeader,
    transactions: Vec<Transaction>,
}

impl Parse for Block {
    fn parse(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (header, bytes) = Parse::parse(bytes)?;
        let (transactions, bytes) = Parse::parse(bytes)?;

        let block = Block {
            header, transactions
        };

        Ok((block, bytes))
    }
}

#[derive(Debug)]
struct OutPoint {
    txid: [u8; 32],
    vout: u32,
}

impl OutPoint {
    fn is_coinbase(&self) -> bool {
        self.txid == [0; 32] && self.vout == 0xFFFFFFFF
    }
}

impl Parse for OutPoint {
    fn parse(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (txid, bytes) = Parse::parse(bytes)?;
        let (vout, bytes) = Parse::parse(bytes)?;

        let outpoint = OutPoint {
            txid, vout
        };

        Ok((outpoint, bytes))
    }
}

impl<T: Parse> Parse for Vec<T> {
    fn parse(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (len, mut bytes) = VarInt::parse(&bytes)?;
        //let data = bytes[..(len.0 as usize)].iter().cloned().collect();
        let mut data = Vec::new();
        for _ in 0..(len.0 as usize) {
            let (item, remainder) = T::parse(bytes)?;
            data.push(item);
            bytes = remainder;
        }

        Ok((data, bytes))
    }
}

#[derive(Debug)]
struct TxIn {
    previous_output: OutPoint,
    script_sig: Script,
    sequence: u32,
}

#[derive(Debug)]
enum OpCode {
    Return,
    Dup,
    Equal,
    CheckSig,
    Hash160,
    EqualVerify,
    Push(Vec<u8>),
}

impl Parse for OpCode {
    fn parse(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        dbg!(bytes[0]);
        dbg!(match bytes[0] {
            v @ 1..=75 => {
                let data = bytes[1..(v as usize + 1)].iter().cloned().collect();
                Ok((OpCode::Push(data), &bytes[(v as usize + 1)..]))
            },
            76 => {
                let len = bytes[1] as usize;
                let data = bytes[2..(len + 2)].iter().cloned().collect();
                Ok((OpCode::Push(data), &bytes[(len + 2)..]))
            },

            106 => Ok((OpCode::Return, &bytes[1..])),
            118 => Ok((OpCode::Dup, &bytes[1..])),
            135 => Ok((OpCode::Equal, &bytes[1..])),

            136 => Ok((OpCode::EqualVerify, &bytes[1..])),
            169 => Ok((OpCode::Hash160, &bytes[1..])),
            172 => Ok((OpCode::CheckSig, &bytes[1..])),

            _ => todo!()
        })
    }
}

#[derive(Debug)]
struct Script(Vec<OpCode>);

impl Parse for Script {
    fn parse(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (len, bytes) = VarInt::parse(bytes)?;
        let mut script_bytes = &bytes[..len.0 as usize];
        let mut opcodes = Vec::new();
        while !script_bytes.is_empty() {
            let (opcode, bytes) = OpCode::parse(script_bytes)?;
            script_bytes = bytes;
            opcodes.push(opcode);
        }

        Ok((Script(opcodes), &bytes[len.0 as usize..]))
    }
}

impl Parse for TxIn {
    fn parse(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (previous_output, bytes) = OutPoint::parse(bytes)?;
        let (script_sig, bytes) = if previous_output.is_coinbase() {
            let (_, bytes) = VarInt::parse(bytes)?;
            (Script(vec![]), bytes)
        } else {
            Parse::parse(bytes)?
        };
        let (sequence, bytes) = Parse::parse(bytes)?;

        let txin = TxIn {
            previous_output,
            script_sig,
            sequence
        };

        Ok((txin, bytes))
    }
}

#[derive(Debug)]
struct TxOut {
    value: u64,
    script_pubkey: Script,
}

impl Parse for TxOut {
    fn parse(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (value, bytes) = Parse::parse(bytes)?;
        let (script_pubkey, bytes) = Parse::parse(bytes)?;

        let txout = TxOut {
            value, script_pubkey
        };

        Ok((txout, bytes))
    }
}

#[derive(Debug)]
struct Transaction {
    version: u32,
    inputs: Vec<TxIn>,
    outputs: Vec<TxOut>,
    locktime: u32,
}

impl Parse for Transaction {
    fn parse(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (version, bytes) = Parse::parse(bytes)?;
        let (inputs, bytes) = Parse::parse(bytes)?;
        let (outputs, bytes) = Parse::parse(bytes)?;
        let (locktime, bytes) = Parse::parse(bytes)?;

        let tx = Transaction {
            version, inputs, outputs, locktime
        };

        Ok((tx, bytes))
    }
}

fn main() -> Result<(), Error> {
    let block_bytes = from_hex(&BLOCK)?;
    dbg!(block_bytes.len());

    let (block, bytes) = Block::parse(&block_bytes)?;
    assert!(bytes.len() == 0);
    dbg!(block.header);
    dbg!(block.transactions.len());
    dbg!(&block.transactions[0]);

    Ok(())
}