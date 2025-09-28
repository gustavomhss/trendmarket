//! Erros padronizados do AMM
use core::fmt;

macro_rules! amm_error_contract {
    (
        $(
            $variant:ident => {
                code: $code:expr,
                message: $message:expr,
                http_status: $status:expr
            }
        ),+ $(,)?
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum AmmError {
            $( $variant, )+
        }

        #[derive(Debug, Clone, Copy)]
        pub struct AmmErrorDescriptor {
            pub variant: AmmError,
            pub code: &'static str,
            pub message: &'static str,
            pub http_status: Option<u16>,
        }

        impl AmmError {
            pub const ALL_VARIANTS: [AmmError; amm_error_contract!(@len $($variant),+)] = [
                $( AmmError::$variant, )+
            ];

            pub const fn error_code(self) -> &'static str {
                match self {
                    $( AmmError::$variant => $code, )+
                }
            }

            pub const fn user_message(self) -> &'static str {
                match self {
                    $( AmmError::$variant => $message, )+
                }
            }

            pub const fn http_status(self) -> Option<u16> {
                match self {
                    $( AmmError::$variant => Some($status), )+
                }
            }

            pub const fn variant_name(self) -> &'static str {
                match self {
                    $( AmmError::$variant => stringify!($variant), )+
                }
            }

            pub const fn descriptors() -> &'static [AmmErrorDescriptor] {
                &AMM_ERROR_DESCRIPTORS
            }
        }

        pub const AMM_ERROR_DESCRIPTORS: [AmmErrorDescriptor; amm_error_contract!(@len $($variant),+)] = [
            $( AmmErrorDescriptor {
                variant: AmmError::$variant,
                code: $code,
                message: $message,
                http_status: Some($status),
            }, )+
        ];
    };

    (@len $($variant:ident),+) => {
        <[()]>::len(&[ $(amm_error_contract!(@unit $variant)),+ ])
    };

    (@unit $variant:ident) => { () };
}

amm_error_contract! {
    ZeroAmount => {
        code: "CE-AMM-0001",
        message: "Input amount must be greater than zero.",
        http_status: 400
    },
    ZeroReserve => {
        code: "CE-AMM-0002",
        message: "Reserves must stay above zero.",
        http_status: 400
    },
    MinReserveBreached => {
        code: "CE-AMM-0003",
        message: "Operation would breach the minimum reserve.",
        http_status: 409
    },
    Overflow => {
        code: "CE-AMM-0004",
        message: "Numerical overflow or underflow detected.",
        http_status: 500
    },
    InputTooSmall => {
        code: "CE-AMM-0005",
        message: "Effective input amount is too small.",
        http_status: 400
    },
    InvalidFee => {
        code: "CE-AMM-0006",
        message: "Fee ppm must be at most 1,000,000.",
        http_status: 400
    }
}

impl fmt::Display for AmmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.user_message())
    }
}

impl std::error::Error for AmmError {}
