use crate::evm_core::utils::{Sign, I256};
use primitive_types::U256;

pub fn slt(op1: U256, op2: U256) -> U256 {
    let op1: I256 = op1.into();
    let op2: I256 = op2.into();

    if op1.lt(&op2) {
        U256::one()
    } else {
        U256::zero()
    }
}

pub fn sgt(op1: U256, op2: U256) -> U256 {
    let op1: I256 = op1.into();
    let op2: I256 = op2.into();

    if op1.gt(&op2) {
        U256::one()
    } else {
        U256::zero()
    }
}

pub fn iszero(op1: U256) -> U256 {
    if op1 == U256::zero() {
        U256::one()
    } else {
        U256::zero()
    }
}

pub fn not(op1: U256) -> U256 {
    !op1
}

pub fn byte(op1: U256, op2: U256) -> U256 {
    let mut ret = U256::zero();

    for i in 0..256 {
        if i < 8 && op1 < 32.into() {
            let o: usize = op1.as_usize();
            let t = 255 - (7 - i + 8 * o);
            let bit_mask = U256::one() << t;
            let value = (op2 & bit_mask) >> t;
            ret = ret.overflowing_add(value << i).0;
        }
    }

    ret
}

pub fn shl(shift: U256, value: U256) -> U256 {
    let ret = if value == U256::zero() || shift >= U256::from(256) {
        U256::zero()
    } else {
        let shift: u64 = shift.as_u64();
        value << shift as usize
    };

    ret
}

pub fn shr(shift: U256, value: U256) -> U256 {
    let ret = if value == U256::zero() || shift >= U256::from(256) {
        U256::zero()
    } else {
        let shift: u64 = shift.as_u64();
        value >> shift as usize
    };

    ret
}

pub fn sar(shift: U256, value: U256) -> U256 {
    let value = I256::from(value);

    let ret = if value == I256::zero() || shift >= U256::from(256) {
        let I256(sign, _) = value;
        match sign {
            // value is 0 or >=1, pushing 0
            Sign::Plus | Sign::NoSign => U256::zero(),
            // value is <0, pushing -1
            Sign::Minus => I256(Sign::Minus, U256::one()).into(),
        }
    } else {
        let shift: u64 = shift.as_u64();

        match value.0 {
            Sign::Plus | Sign::NoSign => value.1 >> shift as usize,
            Sign::Minus => {
                let shifted = ((value.1.overflowing_sub(U256::one()).0) >> shift as usize)
                    .overflowing_add(U256::one())
                    .0;
                I256(Sign::Minus, shifted).into()
            }
        }
    };

    ret
}
