use atomic_refcell::AtomicRefCell;
use basedrop::{Collector, Handle, Shared, SharedCell};

pub mod node;

mod graph;
mod pool;
mod schedule;

pub use graph::{NodeID, PortType};
pub use node::AudioGraphNode;
pub use pool::{MonoAudioPortBuffer, StereoAudioPortBuffer};
pub use schedule::{AudioGraphTask, ProcInfo};

use graph::Graph;
use pool::GraphResourcePool;
use schedule::Schedule;

pub const MAX_BLOCKSIZE: usize = 128;

pub struct GraphState {
    shared_graph_state: Shared<SharedCell<CompiledGraph>>,
    resource_pool_state: GraphResourcePool,
    graph: Graph,

    sample_rate: f32,
    coll_handle: Handle,
}

impl GraphState {
    pub fn new(sample_rate: f32, coll_handle: Handle) -> (Self, Shared<SharedCell<CompiledGraph>>) {
        let collector = Collector::new();

        let (shared_graph_state, resource_pool_state) =
            CompiledGraph::new(collector.handle(), sample_rate);
        let rt_shared_state = Shared::clone(&shared_graph_state);

        (
            Self {
                shared_graph_state,
                resource_pool_state,
                graph: Graph::new(),
                sample_rate,
                coll_handle,
            },
            rt_shared_state,
        )
    }

    // TODO: Some way to modify the delay compensation of nodes, which will cause the graph to recompile.

    // We are using a closure for all modifications to the graph instead of using individual methods to act on
    // the graph. This is so the graph only gets compiled once after the user is done, instead of being recompiled
    // after every method.
    pub fn modify_graph<F: FnOnce(GraphRef<'_>)>(&mut self, f: F) {
        let graph_state_ref = GraphRef {
            resource_pool: &mut self.resource_pool_state,
            graph: &mut self.graph,
        };

        (f)(graph_state_ref);

        self.compile_graph();
    }

    fn compile_graph(&mut self) {
        let mut tasks = Vec::<AudioGraphTask>::new();

        // Manually setting up the task for now. Later we will use the actual `audio_graph` crate
        // to compile the graph schedule for us.

        // We will need at-least two stereo buffers.
        if self.resource_pool_state.stereo_audio_buffers.len() < 2 {
            self.resource_pool_state.add_stereo_audio_port_buffers(
                2 - self.resource_pool_state.stereo_audio_buffers.len(),
            );
        }

        let buffer_1 = &self.resource_pool_state.stereo_audio_buffers[0];
        let buffer_2 = &self.resource_pool_state.stereo_audio_buffers[1];

        // Add the stereo timeline track node.
        tasks.push(AudioGraphTask::Node {
            node: Shared::clone(&self.resource_pool_state.nodes[0]),
            mono_audio_in_buffers: vec![],
            mono_audio_out_buffers: vec![],
            stereo_audio_in_buffers: vec![],
            stereo_audio_out_buffers: vec![Shared::clone(buffer_1)],
        });

        let new_schedule = Schedule::new(tasks, self.sample_rate, Shared::clone(buffer_1));

        let new_shared_state = Shared::new(
            &self.coll_handle,
            CompiledGraph {
                resource_pool: AtomicRefCell::new(GraphResourcePool::clone(
                    &self.resource_pool_state,
                )),
                schedule: AtomicRefCell::new(new_schedule),
            },
        );

        // This new state will be available to the rt thread at the top of the next process loop.
        self.shared_graph_state.set(new_shared_state);
    }
}

pub struct GraphRef<'a> {
    resource_pool: &'a mut GraphResourcePool,
    graph: &'a mut Graph,
}

impl<'a> GraphRef<'a> {
    pub fn add_new_node(&mut self, node: Box<dyn AudioGraphNode>) -> NodeID {
        let node_id = self.graph.add_new_node(&node);

        self.resource_pool.add_node(node);

        node_id
    }

    // TODO: Return custom error type.
    /// Remove a node from the graph.
    ///
    /// This will automatically remove all connections to this node as well.
    pub fn remove_node(&mut self, node_id: &NodeID) -> Result<(), ()> {
        if let Ok(index) = self.graph.remove_node(node_id) {
            // This shouldn't panic because the `graph` found a valid index.
            self.resource_pool.remove_node(index).unwrap();

            Ok(())
        } else {
            Err(())
        }
    }

    // Replace a node with another node while attempting to keep existing connections.
    pub fn replace_node(
        &mut self,
        node_id: &NodeID,
        new_node: Box<dyn AudioGraphNode>,
    ) -> Result<(), ()> {
        if let Ok(index) = self.graph.replace_node(node_id, &new_node) {
            // This shouldn't panic because the `graph` found a valid index.
            self.resource_pool.replace_node(index, new_node).unwrap();

            Ok(())
        } else {
            Err(())
        }
    }

    // TODO: Return custom error type.
    /// Add a connection between nodes.
    pub fn add_port_connection(
        &mut self,
        port_type: PortType,
        source_node_id: &NodeID,
        source_node_port_index: usize,
        dest_node_id: &NodeID,
        dest_node_port_index: usize,
    ) -> Result<(), ()> {
        self.graph.add_port_connection(
            port_type,
            source_node_id,
            source_node_port_index,
            dest_node_id,
            dest_node_port_index,
        )
    }

    // TODO: Return custom error type.
    /// Remove a connection between nodes.
    pub fn remove_port_connection(
        &mut self,
        port_type: PortType,
        source_node_id: &NodeID,
        source_node_port_index: usize,
        dest_node_id: &NodeID,
        dest_node_port_index: usize,
    ) -> Result<(), ()> {
        self.graph.remove_port_connection(
            port_type,
            source_node_id,
            source_node_port_index,
            dest_node_id,
            dest_node_port_index,
        )
    }
}

pub struct CompiledGraph {
    pub resource_pool: AtomicRefCell<GraphResourcePool>,
    pub schedule: AtomicRefCell<Schedule>,
}

impl CompiledGraph {
    fn new(
        coll_handle: Handle,
        sample_rate: f32,
    ) -> (Shared<SharedCell<CompiledGraph>>, GraphResourcePool) {
        let mut resource_pool = GraphResourcePool::new(coll_handle.clone());
        // Allocate a buffer for the master output.
        resource_pool.add_stereo_audio_port_buffers(1);

        let master_out = Shared::clone(&resource_pool.stereo_audio_buffers[0]);

        (
            Shared::new(
                &coll_handle,
                SharedCell::new(Shared::new(
                    &coll_handle,
                    CompiledGraph {
                        resource_pool: AtomicRefCell::new(GraphResourcePool::clone(&resource_pool)),
                        schedule: AtomicRefCell::new(Schedule::new(
                            vec![],
                            sample_rate,
                            master_out,
                        )),
                    },
                )),
            ),
            resource_pool,
        )
    }

    /// Where the magic happens! Only to be used by the rt thread.
    pub fn process<T: cpal::Sample>(&self, mut cpal_out: &mut [T]) {
        // Should not panic because the non-rt thread only mutates its own copy of these resources. It sends
        // a copy to the rt thread via a SharedCell.
        let resource_pool = &mut *AtomicRefCell::borrow_mut(&self.resource_pool);

        // Should not panic because the non-rt thread always creates a new schedule every time before sending
        // it to the rt thread via a SharedCell.
        let schedule = &mut *AtomicRefCell::borrow_mut(&self.schedule);

        // Assume output is stereo for now.
        let mut frames_left = cpal_out.len() / 2;

        // Process in blocks.
        while frames_left > 0 {
            let frames = frames_left.min(MAX_BLOCKSIZE);

            resource_pool.clear_all_buffers();

            schedule.process(frames);

            schedule.copy_master_output_to_cpal(&mut cpal_out[0..(frames * 2)]);

            cpal_out = &mut cpal_out[(frames * 2)..];
            frames_left -= frames;
        }
    }
}