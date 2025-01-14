pub mod graph;

/// Responsibilities:
/// - Manage the RenderGraph object
/// - Build and schedule passes based on dependencies
/// - Record command buffers in the correct order
pub struct RenderGraphContext;