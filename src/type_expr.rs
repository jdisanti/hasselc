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
    UnresolvedNumber,
    Void,
}

impl BaseType {
    pub fn as_native(&self) -> Option<NativeType> {
        use self::BaseType::*;
        match *self {
            Bool => Some(NativeType::U8),
            Pointer(_) => Some(NativeType::U16),
            U16 => Some(NativeType::U16),
            U8 => Some(NativeType::U8),
            UnresolvedNumber => None,
            Void => None,
        }
    }

    pub fn can_index_array(&self) -> bool {
        self.can_assign_into(&BaseType::U16)
    }

    pub fn can_assign_into(&self, into: &BaseType) -> bool {
        use self::BaseType::*;
        if self == into {
            true
        } else {
            match *self {
                Pointer(_) => match *into {
                    U16 => true,
                    _ => false,
                },
                U16 => match *into {
                    Pointer(_) | UnresolvedNumber => true,
                    _ => false,
                },
                U8 => match *into {
                    Pointer(_) | U16 | UnresolvedNumber => true,
                    _ => false,
                },
                Bool => match *into {
                    U8 | U16 | UnresolvedNumber => true,
                    _ => false,
                },
                Void => false,
                UnresolvedNumber => match *into {
                    U8 | U16 | Pointer(_) => true,
                    _ => false,
                },
            }
        }
    }

    pub fn can_cast_into(&self, into: &BaseType) -> bool {
        use self::BaseType::*;
        if self == into {
            true
        } else {
            match *self {
                Pointer(_) => match *into {
                    U16 | U8 | Bool => true,
                    _ => false,
                },
                U16 => match *into {
                    Pointer(_) | U8 | Bool => true,
                    _ => false,
                },
                U8 => match *into {
                    Pointer(_) | U16 | Bool => true,
                    _ => false,
                },
                Bool => match *into {
                    U8 | U16 => true,
                    _ => false,
                },
                Void => false,
                UnresolvedNumber => match *into {
                    U8 | U16 | Pointer(_) => true,
                    _ => false,
                },
            }
        }
    }

    pub fn can_compare(&self, with: &BaseType) -> bool {
        use self::BaseType::*;
        if self == with {
            true
        } else {
            match *self {
                Pointer(_) => match *with {
                    U16 => true,
                    _ => false,
                },
                U16 => match *with {
                    Pointer(_) | UnresolvedNumber => true,
                    _ => false,
                },
                U8 => match *with {
                    UnresolvedNumber => true,
                    _ => false,
                },
                Bool => false,
                Void => false,
                UnresolvedNumber => match *with {
                    U8 | U16 => true,
                    _ => false,
                },
            }
        }
    }

    pub fn underlying_type(&self) -> Option<BaseType> {
        use self::BaseType::*;
        match *self {
            Pointer(ref base_type) => Some(*(&*base_type).clone()),
            _ => None,
        }
    }

    pub fn size(&self) -> Option<usize> {
        use self::BaseType::*;
        match *self {
            Bool => Some(1),
            Pointer(_) => Some(2),
            U16 => Some(2),
            U8 => Some(1),
            Void => Some(0),
            UnresolvedNumber => None,
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
            } else if left_size.is_none() {
                Some(right.clone())
            } else if right_size.is_none() {
                Some(left.clone())
            } else if left_size.unwrap() > right_size.unwrap() {
                Some(left.clone())
            } else {
                Some(right.clone())
            }
        } else {
            None
        }
    }
}

pub enum TypeExpression {
    Arithmetic(Box<TypeExpression>, Box<TypeExpression>),
    Assign(BaseType, Box<TypeExpression>),
    Cast(BaseType, Box<TypeExpression>),
    Comparison(Box<TypeExpression>, Box<TypeExpression>),
    Index(Box<TypeExpression>, Box<TypeExpression>),
    Unit(BaseType),
}

impl TypeExpression {
    pub fn resolve(&self) -> Option<BaseType> {
        use self::TypeExpression::*;

        match *self {
            Arithmetic(ref left, ref right) => {
                let left_type = left.resolve()?;
                let right_type = right.resolve()?;
                BaseType::choose_type(&left_type, &right_type)
            }
            Assign(ref into, ref value) => {
                let value_type = value.resolve()?;
                if value_type.can_assign_into(into) {
                    Some(into.clone())
                } else {
                    None
                }
            }
            Cast(ref into, ref value) => {
                let value_type = value.resolve()?;
                if value_type.can_cast_into(into) {
                    Some(into.clone())
                } else {
                    None
                }
            }
            Comparison(ref left, ref right) => {
                let left_type = left.resolve()?;
                let right_type = right.resolve()?;
                if left_type.can_compare(&right_type) {
                    Some(BaseType::Bool)
                } else {
                    None
                }
            }
            Index(ref pointer, ref index) => {
                let pointer_type = pointer.resolve()?;
                let index_type = index.resolve()?;
                if let BaseType::Pointer(underlying_type) = pointer_type {
                    if index_type.can_index_array() {
                        Some((*underlying_type).clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Unit(ref base_type) => Some(base_type.clone()),
        }
    }

    pub fn is_equivalent(&self, base_type: &BaseType) -> bool {
        match self.resolve() {
            Some(resolved) => resolved.can_compare(base_type),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TypeExpression::*;
    use super::BaseType::*;

    #[test]
    fn test_expressions() {
        let expr = Assign(
            U16,
            Box::new(Arithmetic(
                Box::new(Unit(UnresolvedNumber)),
                Box::new(Unit(UnresolvedNumber)),
            )),
        );
        assert_eq!(Some(U16), expr.resolve());

        let expr = Assign(
            U8,
            Box::new(Arithmetic(
                Box::new(Unit(UnresolvedNumber)),
                Box::new(Unit(UnresolvedNumber)),
            )),
        );
        assert_eq!(Some(U8), expr.resolve());

        let expr = Assign(
            Pointer(Box::new(U8)),
            Box::new(Arithmetic(
                Box::new(Unit(UnresolvedNumber)),
                Box::new(Unit(UnresolvedNumber)),
            )),
        );
        assert_eq!(Some(Pointer(Box::new(U8))), expr.resolve());
    }

    #[test]
    fn test_arithmetic() {
        let expr = Arithmetic(Box::new(Unit(U8)), Box::new(Unit(U8)));
        assert_eq!(Some(U8), expr.resolve());

        let expr = Arithmetic(Box::new(Unit(U16)), Box::new(Unit(U8)));
        assert_eq!(Some(U16), expr.resolve());

        let expr = Arithmetic(Box::new(Unit(U16)), Box::new(Unit(U16)));
        assert_eq!(Some(U16), expr.resolve());

        let expr = Arithmetic(Box::new(Unit(U8)), Box::new(Unit(U16)));
        assert_eq!(Some(U16), expr.resolve());

        let expr = Arithmetic(Box::new(Unit(U8)), Box::new(Unit(Bool)));
        assert_eq!(Some(U8), expr.resolve());

        let expr = Arithmetic(Box::new(Unit(Bool)), Box::new(Unit(U8)));
        assert_eq!(Some(U8), expr.resolve());

        let expr = Arithmetic(Box::new(Unit(U16)), Box::new(Unit(Bool)));
        assert_eq!(Some(U16), expr.resolve());

        let expr = Arithmetic(Box::new(Unit(Bool)), Box::new(Unit(U16)));
        assert_eq!(Some(U16), expr.resolve());

        let expr = Arithmetic(Box::new(Unit(U8)), Box::new(Unit(Void)));
        assert_eq!(None, expr.resolve());

        let expr = Arithmetic(Box::new(Unit(Void)), Box::new(Unit(U8)));
        assert_eq!(None, expr.resolve());

        let expr = Arithmetic(Box::new(Unit(U16)), Box::new(Unit(Void)));
        assert_eq!(None, expr.resolve());

        let expr = Arithmetic(Box::new(Unit(Void)), Box::new(Unit(U16)));
        assert_eq!(None, expr.resolve());
    }

    #[test]
    fn test_assign() {
        let expr = Assign(U8, Box::new(Unit(U8)));
        assert_eq!(Some(U8), expr.resolve());

        let expr = Assign(U8, Box::new(Unit(UnresolvedNumber)));
        assert_eq!(Some(U8), expr.resolve());

        let expr = Assign(U8, Box::new(Unit(U16)));
        assert_eq!(None, expr.resolve());

        let expr = Assign(U16, Box::new(Unit(U8)));
        assert_eq!(Some(U16), expr.resolve());

        let expr = Assign(U16, Box::new(Unit(Pointer(Box::new(U8)))));
        assert_eq!(Some(U16), expr.resolve());

        let expr = Assign(Pointer(Box::new(U8)), Box::new(Unit(U16)));
        assert_eq!(Some(Pointer(Box::new(U8))), expr.resolve());
    }

    #[test]
    fn test_cast() {
        let expr = Cast(U8, Box::new(Unit(U8)));
        assert_eq!(Some(U8), expr.resolve());

        let expr = Cast(Bool, Box::new(Unit(U8)));
        assert_eq!(Some(Bool), expr.resolve());

        let expr = Cast(Pointer(Box::new(U8)), Box::new(Unit(U8)));
        assert_eq!(Some(Pointer(Box::new(U8))), expr.resolve());

        let expr = Cast(Bool, Box::new(Unit(Pointer(Box::new(U8)))));
        assert_eq!(Some(Bool), expr.resolve());
    }

    #[test]
    fn test_comparison() {
        let expr = Comparison(Box::new(Unit(U8)), Box::new(Unit(U8)));
        assert_eq!(Some(Bool), expr.resolve());

        let expr = Comparison(Box::new(Unit(U16)), Box::new(Unit(U8)));
        assert_eq!(None, expr.resolve());

        let expr = Comparison(Box::new(Unit(U8)), Box::new(Unit(U16)));
        assert_eq!(None, expr.resolve());

        let expr = Comparison(Box::new(Unit(U16)), Box::new(Unit(U16)));
        assert_eq!(Some(Bool), expr.resolve());

        let expr = Comparison(Box::new(Unit(Bool)), Box::new(Unit(Bool)));
        assert_eq!(Some(Bool), expr.resolve());

        let expr = Comparison(
            Box::new(Unit(Pointer(Box::new(U8)))),
            Box::new(Unit(Pointer(Box::new(U8)))),
        );
        assert_eq!(Some(Bool), expr.resolve());
    }

    #[test]
    fn test_index() {
        let expr = Arithmetic(
            Box::new(Unit(U8)),
            Box::new(Index(
                Box::new(Unit(Pointer(Box::new(U8)))),
                Box::new(Unit(U16)),
            )),
        );
        assert_eq!(Some(U8), expr.resolve());
    }
}
