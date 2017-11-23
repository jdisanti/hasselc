#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Type {
    U8,
    U16,
    Void,
    ArrayU8,
    Unresolved,
}

impl Type {
    pub fn size(&self) -> usize {
        match *self {
            Type::U8 => 1,
            Type::U16 | Type::ArrayU8 => 2,
            Type::Void => 0,
            Type::Unresolved => unreachable!(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TypedValue {
    UnresolvedInt(i32),
    U8(u8),
    U16(u16),
    ArrayU8(u16),
}

impl TypedValue {
    pub fn get_type(&self) -> Type {
        match *self {
            TypedValue::UnresolvedInt(_) => Type::Void,
            TypedValue::U8(_) => Type::U8,
            TypedValue::U16(_) => Type::U16,
            TypedValue::ArrayU8(_) => Type::ArrayU8,
        }
    }

    pub fn as_u8(&self) -> u8 {
        match *self {
            TypedValue::U8(val) => val,
            _ => panic!("expected u8"),
        }
    }

    pub fn as_u16(&self) -> u16 {
        match *self {
            TypedValue::U16(val) => val,
            _ => panic!("expected u16"),
        }
    }

    pub fn as_ptr(&self) -> u16 {
        match *self {
            TypedValue::ArrayU8(addr) => addr,
            _ => panic!("expected array/pointer"),
        }
    }
}
