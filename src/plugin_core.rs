use serde::json;
use super::plugin_base::{FunctionResult, Plugin, FunctionCallContext, PluginId};
use super::server::{CommandSender, Command};

struct CorePlugin;

impl Plugin for CorePlugin {
    fn name(&self) -> &'static str { "core" }
    // NOCOM(#sirver): rethink this name.
    fn broadcast(&self, _: &json::value::Value) {
    }
    fn id(&self) -> PluginId {
        PluginId::Local("core")
    }
    fn call(&self, context: FunctionCallContext) -> FunctionResult {
        match &context.function as &str {
            "core.exit" => {
                context.commands.send(Command::Shutdown).unwrap();
            },
            "core.broadcast" => {
                context.commands.send(Command::Broadcast(context.args)).unwrap();
            },
            "core.register_function" => {
                let function = context.args.find("name")
                    .and_then(|o| o.as_string())
                    .unwrap().into();
                context.commands.send(
                    Command::RegisterFunction(context.caller, function)).unwrap();
            },
            _ => panic!("{} was called, but is not a core function.", context.function),
        }
        FunctionResult::DONE
    }
}

pub fn register(command_sender: &CommandSender) {
    let core = CorePlugin;
    let id = core.id();

    command_sender.send(Command::PluginConnected(Box::new(CorePlugin))).unwrap();

    // NOCOM(#sirver): ugly repetition.
    command_sender.send(Command::RegisterFunction(id, "core.exit".into())).unwrap();
    command_sender.send(Command::RegisterFunction(id, "core.broadcast".into())).unwrap();
    command_sender.send(Command::RegisterFunction(id, "core.register_function".into())).unwrap();
}