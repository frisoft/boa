use crate::{
    error::JsNativeError,
    vm::{opcode::Operation, CompletionType},
    Context, JsResult, JsString,
};

/// `SetName` implements the Opcode Operation for `Opcode::SetName`
///
/// Operation:
///  - Find a binding on the environment chain and assign its value.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetName;

impl Operation for SetName {
    const NAME: &'static str = "SetName";
    const INSTRUCTION: &'static str = "INST - SetName";

    fn execute(context: &mut Context<'_>) -> JsResult<CompletionType> {
        let index = context.vm.read::<u32>();
        let binding_locator = context.vm.frame().code_block.bindings[index as usize];
        let value = context.vm.pop();
        if binding_locator.is_silent() {
            return Ok(CompletionType::Normal);
        }
        binding_locator.throw_mutate_immutable(context)?;

        if binding_locator.is_global() {
            if !context.put_value_if_global_poisoned(binding_locator.name(), &value)? {
                let key: JsString = context
                    .interner()
                    .resolve_expect(binding_locator.name().sym())
                    .into_common(false);
                let exists = context.global_bindings_mut().contains_key(&key);

                if !exists && context.vm.frame().code_block.strict {
                    return Err(JsNativeError::reference()
                        .with_message(format!(
                            "assignment to undeclared variable {}",
                            key.to_std_string_escaped()
                        ))
                        .into());
                }

                let success = crate::object::internal_methods::global::global_set_no_receiver(
                    &key.clone().into(),
                    value,
                    context,
                )?;

                if !success && context.vm.frame().code_block.strict {
                    return Err(JsNativeError::typ()
                        .with_message(format!(
                            "cannot set non-writable property: {}",
                            key.to_std_string_escaped()
                        ))
                        .into());
                }
            }
        } else if !context.put_value_if_initialized(
            binding_locator.environment_index(),
            binding_locator.binding_index(),
            binding_locator.name(),
            value,
        )? {
            return Err(JsNativeError::reference()
                .with_message(format!(
                    "cannot access '{}' before initialization",
                    context
                        .interner()
                        .resolve_expect(binding_locator.name().sym())
                ))
                .into());
        }
        Ok(CompletionType::Normal)
    }
}