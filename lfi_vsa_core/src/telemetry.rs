// ============================================================
// LFI Delta Telemetry — Workflow Delta (The Watchdog)
// Section 5: Universal Telemetry Rule
// All logs are structurally isolated for production stripping.
// ============================================================

/// Core telemetry macro. Emits `[DEBUGLOG][file:line] - message`.
/// Production stripping: comment out the body or gate behind a feature flag.
#[macro_export]
macro_rules! debuglog {
    ($($arg:tt)*) => {
        println!(
            "[DEBUGLOG][{}:{}] - {}",
            file!(),
            line!(),
            format_args!($($arg)*)
        );
    };
}

/// Value-inspection variant. Emits debug representation of a value.
/// Usage: `debuglog_val!("label", &my_var);`
#[macro_export]
macro_rules! debuglog_val {
    ($label:expr, $val:expr) => {
        println!(
            "[DEBUGLOG][{}:{}] - {} = {:#?}",
            file!(),
            line!(),
            $label,
            $val
        );
    };
}
