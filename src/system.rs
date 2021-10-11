use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::rc::Rc;

use rand::prelude::*;

use crate::net::*;
use crate::node::*;
use crate::sim::*;

#[derive(Debug, Clone)]
pub enum SysEvent<M: Debug + Clone> {
    MessageSend {
        msg: M,
        src: ActorId,
        dest: ActorId,
    },
    MessageReceive {
        msg: M,
        src: ActorId,
        dest: ActorId,
    },
    LocalMessageReceive {
        msg: M,
    },
    TimerSet {
        name: String,
        delay: f64,
    },
    TimerFired {
        name: String,
    },
}

pub struct System<M: Debug + Clone> {
    sim: Simulation<SysEvent<M>>,
    net: Rc<RefCell<Network>>,
    nodes: HashMap<String, Rc<RefCell<NodeActor<M>>>>,
    node_ids: Vec<String>,
    crashed_nodes: HashSet<String>,
}

impl<M: Debug + Clone + 'static> System<M> {
    pub fn new() -> Self {
        let seed: u64 = thread_rng().gen_range(1..1_000_000);
        println!("Seed: {}", seed);
        System::with_seed(seed)
    }

    pub fn with_seed(seed: u64) -> Self {
        let mut sim = Simulation::<SysEvent<M>>::new(seed);
        let net = Rc::new(RefCell::new(Network::new()));
        sim.add_actor("net", net.clone());
        Self {
            sim,
            net,
            nodes: HashMap::new(),
            node_ids: Vec::new(),
            crashed_nodes: HashSet::new(),
        }
    }

    pub fn add_node(&mut self, node: Rc<RefCell<dyn Node<M>>>) {
        let id = node.borrow().id().to_string();
        let actor = Rc::new(RefCell::new(NodeActor::new(node)));
        self.sim.add_actor(&id, actor.clone());
        self.nodes.insert(id.clone(), actor);
        self.node_ids.push(id.clone());
        self.add_timer(&id, "init");
    }

    pub fn add_timer(&mut self, node_id: &str, name: &str) {
        self.sim.add_event(
            SysEvent::TimerFired { name: name.to_string() },
            ActorId::from(node_id),
            ActorId::from(node_id),
            0.0,
        );
    }

    pub fn get_node_ids(&self) -> Vec<String> {
        self.node_ids.clone()
    }

    pub fn crash_node(&mut self, node_id: &str) {
        println!("{:>9.3} {:>10} CRASHED!", self.sim.time(), node_id);
        self.crashed_nodes.insert(node_id.to_string());
        let mut node = self.nodes.get(node_id).unwrap().borrow_mut();
        node.crash();
        self.net.borrow_mut().node_crashed(node_id);
    }

    pub fn node_is_crashed(&self, node_id: &str) -> bool {
        self.crashed_nodes.contains(node_id)
    }

    pub fn node_count(&self) -> u32 {
        self.nodes.len() as u32
    }

    pub fn set_delay(&mut self, delay: f64) {
        self.net.borrow_mut().set_delay(delay);
    }

    pub fn set_delays(&mut self, min_delay: f64, max_delay: f64) {
        self.net.borrow_mut().set_delays(min_delay, max_delay);
    }

    pub fn set_drop_rate(&mut self, drop_rate: f64) {
        self.net.borrow_mut().set_drop_rate(drop_rate);
    }

    pub fn set_dupl_rate(&mut self, dupl_rate: f64) {
        self.net.borrow_mut().set_dupl_rate(dupl_rate);
    }

    pub fn drop_incoming(&mut self, node_id: &str) {
        self.net.borrow_mut().drop_incoming(node_id);
    }

    pub fn pass_incoming(&mut self, node_id: &str) {
        self.net.borrow_mut().pass_incoming(node_id);
    }

    pub fn drop_outgoing(&mut self, node_id: &str) {
        self.net.borrow_mut().drop_outgoing(node_id);
    }

    pub fn pass_outgoing(&mut self, node_id: &str) {
        self.net.borrow_mut().pass_outgoing(node_id);
    }

    pub fn disconnect_node(&mut self, node_id: &str) {
        self.net.borrow_mut().disconnect_node(node_id);
    }

    pub fn connect_node(&mut self, node_id: &str) {
        self.net.borrow_mut().connect_node(node_id);
    }

    pub fn disable_link(&mut self, from: &str, to: &str) {
        self.net.borrow_mut().disable_link(from, to);
    }

    pub fn enable_link(&mut self, from: &str, to: &str) {
        self.net.borrow_mut().enable_link(from, to);
    }


    pub fn enable_between(&mut self, from: &str, to: &str) {
        self.net.borrow_mut().enable_link(from, to);
        self.net.borrow_mut().enable_link(to, from);
    }

    pub fn disable_all_links(&mut self) {
        for from in &self.node_ids {
            for to in &self.node_ids {
                if from != to {
                    self.net.borrow_mut().disable_link(from, to);
                }
            }
        }
    }

    pub fn enable_all_links(&mut self) {
        for from in &self.node_ids {
            for to in &self.node_ids {
                if from != to {
                    self.net.borrow_mut().enable_link(from, to);
                }
            }
        }
    }

    pub fn make_partition(&mut self, group1: &[&str], group2: &[&str]) {
        self.net.borrow_mut().make_partition(group1, group2);
    }

    pub fn reset_network(&mut self) {
        self.net.borrow_mut().reset_network();
    }

    pub fn get_network_message_count(&self) -> u64 {
        self.net.borrow().get_message_count()
    }

    pub fn send(&mut self, msg: M, src: &str, dest: &str) {
        let event = SysEvent::MessageSend {
            msg,
            src: ActorId::from(src),
            dest: ActorId::from(dest),
        };
        self.sim.add_event(event, ActorId::from(src), ActorId::from("net"), 0.0);
    }

    pub fn send_local(&mut self, msg: M, dest: &str) {
        let src = ActorId::from(&format!("local@{}", dest));
        let dest = ActorId::from(dest);
        let event = SysEvent::LocalMessageReceive { msg };
        self.sim.add_event(event, src, dest, 0.0);
    }

    pub fn step(&mut self) -> bool {
        self.sim.step()
    }

    pub fn steps(&mut self, step_count: u32) {
        self.sim.steps(step_count)
    }

    pub fn step_until_no_events(&mut self) {
        self.sim.step_until_no_events()
    }

    pub fn step_while(&mut self, f: fn(&SysEvent<M>) -> bool) {
        self.sim.step_while(f);
    }

    pub fn get_local_events(&self, node_id: &str) -> Vec<LocalEvent<M>> {
        let node = self.nodes.get(node_id).unwrap().borrow();
        node.get_local_events()
    }

    pub fn count_undelivered_events(&mut self) -> usize {
        self.sim.read_undelivered_events().len()
    }
}