pub trait ToolMetadata {
    type Input;
    type Output;

    fn description() -> &'static str;
    fn signature() -> &'static str;
    fn call(&self, input: Self::Input) ->  Self::Output;
}
