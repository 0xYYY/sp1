use core::borrow::Borrow;
use core::borrow::BorrowMut;
use p3_field::AbstractField;
use p3_field::Field;
use sp1_derive::AlignedBorrow;
use std::mem::size_of;

use crate::air::SP1AirBuilder;
use crate::air::Word;
use crate::bytes::ByteOpcode;
use crate::disassembler::WORD_SIZE;
use crate::runtime::ExecutionRecord;

/// A set of columns needed to compute the or of two words.
///
/// TODO: This is currently not in use, and thus not tested thoroughly yet.
#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct OrOperation<T> {
    /// The result of `x | y`.
    pub value: Word<T>,
}

impl<F: Field> OrOperation<F> {
    pub fn populate(&mut self, record: &mut ExecutionRecord, x: u32, y: u32) -> u32 {
        let expected = x | y;
        let x_bytes = x.to_le_bytes();
        let y_bytes = y.to_le_bytes();
        for i in 0..WORD_SIZE {
            self.value[i] = F::from_canonical_u8(x_bytes[i] | y_bytes[i]);
            record.lookup_or(x_bytes[i], y_bytes[i]);
        }
        expected
    }

    pub fn eval<AB: SP1AirBuilder>(
        builder: &mut AB,
        a: Word<AB::Var>,
        b: Word<AB::Var>,
        cols: OrOperation<AB::Var>,
        is_real: AB::Var,
    ) {
        for i in 0..WORD_SIZE {
            builder.send_byte(
                AB::F::from_canonical_u32(ByteOpcode::OR as u32),
                cols.value[i],
                a[i],
                b[i],
                is_real,
            );
        }
    }
}
