// src/engine/render.rs
use std::any::Any;

pub trait RenderNode: Send + Any {
    fn render(&mut self, ctx: &mut RenderContext);
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct RenderContext {
    pub delta: f32,
}

pub struct RenderGraph {
    nodes: Vec<Box<dyn RenderNode>>,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add<N: RenderNode + 'static>(&mut self, node: N) {
        self.nodes.push(Box::new(node));
    }

    pub fn execute(&mut self, ctx: &mut RenderContext) {
        for node in &mut self.nodes {
            node.render(ctx);
        }
    }

    pub fn first_mut(&mut self) -> Option<&mut Box<dyn RenderNode>> {
        self.nodes.get_mut(0)
    }
}

impl dyn RenderNode {
    pub fn downcast_mut<T: RenderNode + 'static>(
        &mut self,
    ) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }
}
