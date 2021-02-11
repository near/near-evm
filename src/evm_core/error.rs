use crate::evm_core::ExternalOpcode;
#[cfg(not(feature = "std"))]
use alloc::borrow::Cow;

#[cfg(feature = "std")]
use std::borrow::Cow;

pub trait ToStr {
    fn to_str(self) -> &'static str;
}

/// Trap which indicates that an `ExternalOpcode` has to be handled.
pub type Trap = ExternalOpcode;

/// Capture represents the result of execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capture<E, T> {
    /// The machine has exited. It cannot be executed again.
    Exit(E),
    /// The machine has trapped. It is waiting for external information, and can
    /// be executed again.
    Trap(T),
}

/// Exit reason.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "with-codec", derive(codec::Encode, codec::Decode))]
#[cfg_attr(feature = "with-serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExitReason {
    /// Machine has succeeded.
    Succeed(ExitSucceed),
    /// Machine returns a normal EVM error.
    Error(ExitError),
    /// Machine encountered an explict revert.
    Revert(ExitRevert),
    /// Machine encountered an error that is not supposed to be normal EVM
    /// errors, such as requiring too much memory to execute.
    Fatal(ExitFatal),
}

impl ExitReason {
    /// Whether the exit is succeeded.
    pub fn is_succeed(&self) -> bool {
        match self {
            Self::Succeed(_) => true,
            _ => false,
        }
    }

    /// Whether the exit is error.
    pub fn is_error(&self) -> bool {
        match self {
            Self::Error(_) => true,
            _ => false,
        }
    }

    /// Whether the exit is revert.
    pub fn is_revert(&self) -> bool {
        match self {
            Self::Revert(_) => true,
            _ => false,
        }
    }

    /// Whether the exit is fatal.
    pub fn is_fatal(&self) -> bool {
        match self {
            Self::Fatal(_) => true,
            _ => false,
        }
    }
}

/// Exit succeed reason.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "with-codec", derive(codec::Encode, codec::Decode))]
#[cfg_attr(feature = "with-serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExitSucceed {
    /// Machine encountered an explict stop.
    Stopped,
    /// Machine encountered an explict return.
    Returned,
    /// Machine encountered an explict suicide.
    Suicided,
}

impl From<ExitSucceed> for ExitReason {
    fn from(s: ExitSucceed) -> Self {
        Self::Succeed(s)
    }
}

/// Exit revert reason.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "with-codec", derive(codec::Encode, codec::Decode))]
#[cfg_attr(feature = "with-serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExitRevert {
    /// Machine encountered an explict revert.
    Reverted,
}

impl From<ExitRevert> for ExitReason {
    fn from(s: ExitRevert) -> Self {
        Self::Revert(s)
    }
}

/// Exit error reason.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "with-codec", derive(codec::Encode, codec::Decode))]
#[cfg_attr(feature = "with-serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExitError {
    /// Trying to pop from an empty stack.
    StackUnderflow,
    /// Trying to push into a stack over stack limit.
    StackOverflow,
    /// Jump destination is invalid.
    InvalidJump,
    /// An opcode accesses memory region, but the region is invalid.
    InvalidRange,
    /// Encountered the designated invalid opcode.
    DesignatedInvalid,
    /// Call stack is too deep (runtime).
    CallTooDeep,
    /// Create opcode encountered collision (runtime).
    CreateCollision,
    /// Create init code exceeds limit (runtime).
    CreateContractLimit,

    ///	An opcode accesses external information, but the request is off offset
    ///	limit (runtime).
    OutOfOffset,
    /// Execution runs out of gas (runtime).
    OutOfGas,
    /// Not enough fund to start the execution (runtime).
    OutOfFund,

    /// PC underflowed (unused).
    PCUnderflow,
    /// Attempt to create an empty account (runtime, unused).
    CreateEmpty,

    /// Other normal errors.
    Other(Cow<'static, str>),
}

impl ToStr for ExitError {
    fn to_str(self) -> &'static str {
        match self {
            ExitError::StackUnderflow => "StackUnderflow",
            ExitError::StackOverflow => "StackOverflow",
            ExitError::InvalidJump => "InvalidJump",
            ExitError::InvalidRange => "InvalidRange",
            ExitError::DesignatedInvalid => "DesignatedInvalid",
            ExitError::CallTooDeep => "CallTooDeep",
            ExitError::CreateCollision => "CreateCollision",
            ExitError::CreateContractLimit => "CreateContractLimit",
            ExitError::OutOfOffset => "OutOfOffset",
            ExitError::OutOfGas => "OutOfGas",
            ExitError::OutOfFund => "OutOfFund",
            ExitError::PCUnderflow => "PCUnderflow",
            ExitError::CreateEmpty => "CreateEmpty",
            ExitError::Other(_) => "Other",
        }
    }
}

impl From<ExitError> for ExitReason {
    fn from(s: ExitError) -> Self {
        Self::Error(s)
    }
}

/// Exit fatal reason.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "with-codec", derive(codec::Encode, codec::Decode))]
#[cfg_attr(feature = "with-serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExitFatal {
    /// The operation is not supported.
    NotSupported,
    /// The trap (interrupt) is unhandled.
    UnhandledInterrupt,
    /// The environment explictly set call errors as fatal error.
    CallErrorAsFatal(ExitError),

    /// Other fatal errors.
    Other(Cow<'static, str>),
}

impl ToStr for ExitFatal {
    fn to_str(self) -> &'static str {
        match self {
            ExitFatal::NotSupported => "NotSupported",
            ExitFatal::UnhandledInterrupt => "UnhandledInterrupt",
            ExitFatal::CallErrorAsFatal(_) => "CallErrorAsFatal",
            ExitFatal::Other(_) => "Other",
        }
    }
}

impl From<ExitFatal> for ExitReason {
    fn from(s: ExitFatal) -> Self {
        Self::Fatal(s)
    }
}
