/// Trait for enums that can be serialized to/from database string representation
/// This provides a consistent pattern for all enums stored in SQLite
pub trait DbEnum: Sized {
    /// Convert the enum variant to its database string representation
    fn to_db_string(&self) -> &'static str;

    /// Parse an enum variant from its database string representation
    /// Returns None if the string doesn't match any variant
    fn from_db_string(s: &str) -> Option<Self>;
}

/// Macro to implement DbEnum trait for simple enums
///
/// Usage:
/// ```
/// impl_db_enum!(DeviceType {
///     TempHumiditySensor => "temphumiditysensor",
///     Commander => "commander"
/// });
/// ```
#[macro_export]
macro_rules! impl_db_enum {
    ($enum_name:ident { $($variant:ident => $db_str:expr),+ $(,)? }) => {
        impl $crate::models::db_enum::DbEnum for $enum_name {
            fn to_db_string(&self) -> &'static str {
                match self {
                    $($enum_name::$variant => $db_str),+
                }
            }

            fn from_db_string(s: &str) -> Option<Self> {
                match s {
                    $($db_str => Some($enum_name::$variant)),+,
                    _ => None
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    enum TestEnum {
        Variant1,
        Variant2,
        Variant3,
    }

    impl_db_enum!(TestEnum {
        Variant1 => "variant1",
        Variant2 => "variant2",
        Variant3 => "variant3"
    });

    #[test]
    fn test_to_db_string() {
        assert_eq!(TestEnum::Variant1.to_db_string(), "variant1");
        assert_eq!(TestEnum::Variant2.to_db_string(), "variant2");
        assert_eq!(TestEnum::Variant3.to_db_string(), "variant3");
    }

    #[test]
    fn test_from_db_string_valid() {
        assert_eq!(
            TestEnum::from_db_string("variant1"),
            Some(TestEnum::Variant1)
        );
        assert_eq!(
            TestEnum::from_db_string("variant2"),
            Some(TestEnum::Variant2)
        );
        assert_eq!(
            TestEnum::from_db_string("variant3"),
            Some(TestEnum::Variant3)
        );
    }

    #[test]
    fn test_from_db_string_invalid() {
        assert_eq!(TestEnum::from_db_string("invalid"), None);
        assert_eq!(TestEnum::from_db_string(""), None);
        assert_eq!(TestEnum::from_db_string("VARIANT1"), None);
    }

    #[test]
    fn test_roundtrip() {
        let variants = vec![TestEnum::Variant1, TestEnum::Variant2, TestEnum::Variant3];
        for variant in variants {
            let db_string = variant.to_db_string();
            let parsed = TestEnum::from_db_string(db_string);
            assert_eq!(Some(variant), parsed);
        }
    }
}
