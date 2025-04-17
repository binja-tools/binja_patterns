use std::ops::Range;

use binaryninja::{
    binary_view::BinaryView,
    command::{
        register_command, register_command_for_function, register_command_for_range, Command,
        FunctionCommand, RangeCommand,
    },
    function::Function,
    logger::Logger,
};
use log::LevelFilter;

use crate::patterns::{get_hash, META};

struct CreatePattern;

impl FunctionCommand for CreatePattern {
    fn action(&self, view: &BinaryView, func: &Function) {
        <Self as RangeCommand>::action(self, view, func_to_range(func))
    }

    fn valid(&self, view: &BinaryView, func: &Function) -> bool {
        <Self as RangeCommand>::valid(self, view, func_to_range(func))
    }
}

fn func_to_range(func: &Function) -> Range<u64> {
    func.start()..func.highest_address()
}

impl RangeCommand for CreatePattern {
    fn action(&self, view: &BinaryView, range: Range<u64>) {
        crate::patterns::create_pattern(view, range)
    }

    fn valid(&self, _view: &BinaryView, _range: Range<u64>) -> bool {
        true
    }
}

struct FindPattern;

impl Command for FindPattern {
    fn action(&self, view: &BinaryView) {
        crate::patterns::find_pattern(view)
    }

    fn valid(&self, _view: &BinaryView) -> bool {
        true
    }
}

struct FindNext;

impl Command for FindNext {
    fn action(&self, view: &BinaryView) {
        crate::patterns::find_next(view)
    }

    fn valid(&self, view: &BinaryView) -> bool {
        META.lock().unwrap().get(&get_hash(view)).is_some()
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn CorePluginInit() -> bool {
    Logger::new("patterns").with_level(LevelFilter::Info).init();

    register_command_for_range(
        "Pattern\\Create Pattern for Range",
        "Create a pattern for this address range",
        CreatePattern,
    );
    register_command_for_function(
        "Pattern\\Create Pattern for Function",
        "Create a pattern for this function",
        CreatePattern,
    );
    register_command(
        "Pattern\\Find Pattern",
        "Find an address using a pattern",
        FindPattern,
    );
    register_command(
        "Pattern\\Find Next",
        "Find next address reusing a pattern",
        FindNext,
    );

    true
}
