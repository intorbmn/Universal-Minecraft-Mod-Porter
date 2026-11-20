//! Stable process exit codes for scripts and CI.

/// UMMP's documented CLI process status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ExitCode {
    Ok = 0,
    Error = 1,
    Refused = 2,
    Partial = 3,
    BuildFailed = 4,
    InvalidInput = 5,
}

impl ExitCode {
    #[must_use]
    pub const fn value(self) -> i32 {
        self as i32
    }
}

#[cfg(test)]
mod tests {
    use super::ExitCode;

    #[test]
    fn exit_codes_are_stable() {
        assert_eq!(ExitCode::Ok.value(), 0);
        assert_eq!(ExitCode::Error.value(), 1);
        assert_eq!(ExitCode::Refused.value(), 2);
        assert_eq!(ExitCode::Partial.value(), 3);
        assert_eq!(ExitCode::BuildFailed.value(), 4);
        assert_eq!(ExitCode::InvalidInput.value(), 5);
    }
}
