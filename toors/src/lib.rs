use core::fmt;
use std::any::{Any, TypeId};
use std::collections::HashMap;

pub trait Tool: Any {
    /// Returns a prettified description of the tool.
    fn description(&self) -> &'static str;

    /// Returns full metadata including the tool’s name, its signature (i.e. the input
    /// argument types computed at runtime), and its description.
    fn signature(&self) -> ToolMetadata;
}

#[derive(Clone)]
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub signature: String,
}

impl ToolMetadata {
    pub fn from(name: String, signature: String, description: String) -> Self {
        ToolMetadata {
            name,
            signature,
            description,
        }
    }
}

impl fmt::Display for ToolMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Name: {}\nDescription: {}\nSignature: {}",
            self.name, self.description, self.signature
        )
    }
}

impl fmt::Debug for ToolMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ToolMetadata {{ name: {:?}, description: {:?}, signature: {:?} }}",
            self.name, self.description, self.signature
        )
    }
}

#[derive(Default)]
pub struct ToolCollection {
    tools: Vec<ToolMetadata>,
    instances: HashMap<TypeId, Box<dyn Any>>,
}

impl ToolCollection {
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            instances: HashMap::new(),
        }
    }

    pub fn add<T: Tool + 'static>(&mut self, tool: T) {
        let type_id = TypeId::of::<T>();
        // Use the instance method, which computes metadata at runtime.
        self.tools.push(tool.signature());
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

