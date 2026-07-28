use soroban_sdk::{Env, Val, TryFromVal, ConversionError};

impl TryFromVal<Env, Val> for u8 {
    type Error = ConversionError;
    fn try_from_val(env: &Env, val: &Val) -> Result<Self, Self::Error> {
        u32::try_from_val(env, val).map(|v| v as u8)
    }
}

impl TryFromVal<Env, u8> for Val {
    type Error = ConversionError;
    fn try_from_val(_env: &Env, v: &u8) -> Result<Self, Self::Error> {
        Ok(Val::from(*v as u32))
    }
}

impl TryFromVal<Env, &u8> for Val {
    type Error = ConversionError;
    fn try_from_val(_env: &Env, v: &&u8) -> Result<Self, Self::Error> {
        Ok(Val::from(**v as u32))
    }
}
