use std::any::{Any, TypeId};

pub trait Tool: Any {
    fn description() -> &'static str
    where
        Self: Sized;
    fn signature() -> &'static str
    where
        Self: Sized;
}

#[derive(Clone)]
pub struct ToolMetadata {
    pub description: String,
    pub signature: String, //pub type_id: TypeId,
}

impl ToolMetadata {
    fn from(signature: String, description: String) -> Self {
        ToolMetadata {
            signature,
            description,
        }
    }
}

#[derive(Default)]
pub struct ToolCollection {
    tools: Vec<ToolMetadata>,
    instances: std::collections::HashMap<TypeId, Box<dyn Any>>,
}

impl ToolCollection {
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            instances: std::collections::HashMap::new(),
        }
    }

    pub fn add<T: Tool + 'static>(&mut self, tool: T) {
        let type_id = TypeId::of::<T>();
        self.tools.push(ToolMetadata {
            description: T::description().to_string(),
            signature: T::signature().to_string(),
            //type_id,
        });
        self.instances.insert(type_id, Box::new(tool));
    }

    pub fn list_tools(&self) -> &[ToolMetadata] {
        &self.tools
    }

    pub fn get_tool<T: Tool + 'static>(&self) -> Option<&T> {
        self.instances
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref())
    }
}
