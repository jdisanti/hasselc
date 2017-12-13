use std::fmt::{self, Display};

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum NativeType {
    U8,
    U16,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum BaseType {
    Bool,
    Pointer(Box<BaseType>),
    U16,
    U8,
    Void,
}

impl Display for BaseType {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use self::BaseType::*;
        match *self {
            Bool => write!(f, "bool"),
            Pointer(ref inner) => write!(f, "&[{}]", inner),
            U16 => write!(f, "u16"),
            U8 => write!(f, "u8"),
            Void => write!(f, "void"),
        }
    }
}

impl BaseType {
    pub fn as_native(&self) -> Option<NativeType> {
        use self::BaseType::*;
        match *self {
            U8 | Bool => Some(NativeType::U8),
            U16 | Pointer(_) => Some(NativeType::U16),
            Void => None,
        }
    }

    pub fn is_pointer(&self) -> bool {
        self.can_index()
    }

    pub fn can_index(&self) -> bool {
        match *self {
            BaseType::Pointer(_) => true,
            _ => false,
        }
    }

    pub fn can_index_array(&self) -> bool {
        use self::BaseType::*;
        match *self {
            Bool | Pointer(_) | Void => false,
            U8 | U16 => true,
        }
    }

    pub fn can_assign_into(&self, into: &BaseType) -> bool {
        use self::BaseType::*;
        if self == into {
            true
        } else {
            match *self {
                Pointer(_) => {
                    match *into {
                        U16 => true,
                        _ => false,
                    }
                }
                U16 => {
                    match *into {
                        Pointer(_) => true,
                        _ => false,
                    }
                }
                U8 => {
                    match *into {
                        Pointer(_) | U16 => true,
                        _ => false,
                    }
                }
                Bool => {
                    match *into {
                        U8 | U16 => true,
                        _ => false,
                    }
                }
                Void => false,
            }
        }
    }

    pub fn can_cast_into(&self, into: &BaseType) -> bool {
        use self::BaseType::*;
        if self == into {
            true
        } else {
            match *self {
                Pointer(_) => {
                    match *into {
                        U16 | U8 | Bool => true,
                        _ => false,
                    }
                }
                U16 => {
                    match *into {
                        Pointer(_) | U8 | Bool => true,
                        _ => false,
                    }
                }
                U8 => {
                    match *into {
                        Pointer(_) | U16 | Bool => true,
                        _ => false,
                    }
                }
                Bool => {
                    match *into {
                        U8 | U16 => true,
                        _ => false,
                    }
                }
                Void => false,
            }
        }
    }

    pub fn can_compare(&self, with: &BaseType) -> bool {
        use self::BaseType::*;
        if self == with {
            true
        } else {
            match *self {
                Pointer(_) => {
                    match *with {
                        U16 => true,
                        _ => false,
                    }
                }
                U16 => {
                    match *with {
                        Pointer(_) => true,
                        _ => false,
                    }
                }
                U8 | Bool | Void => false,
            }
        }
    }

    pub fn underlying_type(&self) -> Option<&BaseType> {
        use self::BaseType::*;
        match *self {
            Pointer(ref base_type) => Some(base_type),
            _ => None,
        }
    }

    pub fn size(&self) -> Option<usize> {
        use self::BaseType::*;
        match *self {
            U8 | Bool => Some(1),
            U16 | Pointer(_) => Some(2),
            Void => None,
        }
    }

    pub fn choose_type(left: &BaseType, right: &BaseType) -> Option<BaseType> {
        if left == right {
            Some(left.clone())
        } else if left.can_assign_into(right) || right.can_assign_into(left) {
            let left_size = left.size();
            let right_size = right.size();
            if left_size == right_size {
                if *left == BaseType::Bool {
                    Some(right.clone())
                } else {
                    Some(left.clone())
                }
            } else if right_size.is_none() || left_size.unwrap() > right_size.unwrap() {
                Some(left.clone())
            } else {
                Some(right.clone())
            }
        } else {
            None
        }
    }
}
