use std::collections::HashMap;

use tokio::prelude::*;
use tokio::timer::Interval;

use std::sync::Arc;
use std::time::{Duration, Instant};

use ofp_device::openflow0x01::{DeviceControllerApp, DeviceControllerEvent, DeviceId };
use ofp_device::OfpDevice;
use openflow0x01::message::Message;
use openflow0x01::{ StatsReq, StatsReqType, StatsReqBody, OfpPort, OfpQueue, PortStats, FlowStats,
                    TableStats, QueueStats, Pattern, ALL_TABLES };
use ofp_device::openflow0x01::DeviceController;

fn request_stats(controller: Arc<DeviceController>) {
    controller.list_all_devices().iter().for_each(
        |device| {
            let msg = Message::StatsRequest( StatsReq {
                req_type: StatsReqType::Port,
                flags: 0,
                body: StatsReqBody::PortBody {
                    port_no: OfpPort::OFPPNone as u16
                }
            });
            info!("Requesting port stats for {}", device);
            controller.send_message(device, 0, msg);

            let msg = Message::StatsRequest( StatsReq {
                req_type: StatsReqType::Flow,
                flags: 0,
                body: StatsReqBody::FlowStatsBody {
                    pattern: Pattern::match_all(),
                    table_id: ALL_TABLES,
                    out_port: OfpPort::OFPPNone as u16
                }
            });
            info!("Requesting flow stats for {}", device);
            controller.send_message(device, 0, msg);

            let msg = Message::StatsRequest( StatsReq {
                req_type: StatsReqType::Table,
                flags: 0,
                body: StatsReqBody::TableBody
            });
            info!("Requesting table stats for {}", device);
            controller.send_message(device, 0, msg);

            let msg = Message::StatsRequest( StatsReq {
                req_type: StatsReqType::Queue,
                flags: 0,
                body: StatsReqBody::QueueBody {
                    port_no: OfpPort::OFPPAll as u16,
                    queue_id: OfpQueue::OFPQAll as u32,
                }
            });
            info!("Requesting queue stats for {}", device);
            controller.send_message(device, 0, msg);
        });
}

/// Periodically send stats requests to the device.
pub struct StatsProbing {
    controller: Arc<DeviceController>
}

impl StatsProbing {
    pub fn new(controller: Arc<DeviceController>) -> StatsProbing {
        StatsProbing { controller }
    }

    fn print_port_stats(&self, device_id: &DeviceId, port_stats: &Vec<PortStats>) {
        for port in port_stats {
            self.print_single_port_stats(device_id, port);
        }
    }

    fn print_single_port_stats(&self, device_id: &DeviceId, port_stats: &PortStats) {
        println!("Port stats: {}:{}", device_id, port_stats.port_no);
        println!("tx:{} tx_packets:{} tx_errors:{} tx_dropped:{}",
            port_stats.bytes.tx, port_stats.packets.tx, port_stats.errors.tx, port_stats.dropped.tx);
        println!("rx:{} rx_packets:{} rx_errors:{} rx_dropped:{}",
                 port_stats.bytes.rx, port_stats.packets.rx, port_stats.errors.rx, port_stats.dropped.rx);
    }

    fn print_flow_stats(&self, device_id: &DeviceId, flow_stats: &Vec<FlowStats>) {
        for flow in flow_stats {
            self.print_single_flow_stats(device_id, flow);
        }
    }

    fn print_single_flow_stats(&self, device_id: &DeviceId, flow: &FlowStats) {
        println!("Flow stats: device:{}, cookie:{}, table:{}, pattern:{:?}, priority:{}, idle_timeout:{}, hard_timeout:{}",
                 device_id, flow.cookie, flow.table_id, flow.pattern,
                 flow.priority, flow.idle_timeout, flow.hard_timeout);
        println!("            duration:{}.{} packets:{}, bytes:{}, actions:{:?}",
                 flow.duration_sec, flow.duration_nsec, flow.packet_count,
                 flow.byte_count, flow.actions);
    }

    fn print_table_stats(&self, device_id: &DeviceId, table_stats: &Vec<TableStats>) {
        for table in table_stats {
            self.print_single_table_stats(device_id, table);
        }
    }

    fn print_single_table_stats(&self, device_id: &DeviceId, table: &TableStats) {
        println!("Table stats: device:{}, table:{}, name:{}",
                 device_id, table.table_id, table.name);
        println!("max_entries:{}, active_count:{}, lookup_count:{}, matched_count:{}",
                 table.max_entries, table.active_count, table.lookup_count, table.matched_count);
    }

    fn print_queue_stats(&self, device_id: &DeviceId, queue_stats: &Vec<QueueStats>) {
        for queue in queue_stats {
            self.print_single_queue_stats(device_id, queue);
        }
    }

    fn print_single_queue_stats(&self, device_id: &DeviceId, queue: &QueueStats) {
        println!("Queue stats: device:{}, port:{}, queue:{}, bytes:{}, packets:{}, errors:{}",
                 device_id, queue.port_no, queue.queue_id,
                 queue.tx_bytes, queue.tx_packets, queue.tx_errors);
    }
}

impl DeviceControllerApp for StatsProbing {
    fn event(&mut self, event: Arc<DeviceControllerEvent>) {
        match *event {
            DeviceControllerEvent::PortStats(ref device_id, ref port_stats) => {
                self.print_port_stats(device_id, port_stats);
            },
            DeviceControllerEvent::FlowStats(ref device_id, ref flow_stats) => {
                self.print_flow_stats(device_id, flow_stats);
            },
            DeviceControllerEvent::QueueStats(ref device_id, ref queue_stats) => {
                self.print_queue_stats(device_id, queue_stats);
            },
            DeviceControllerEvent::TableStats(ref device_id, ref table_stats) => {
                self.print_table_stats(device_id, table_stats);
            }
            _ => {}
        }
    }

    fn start(&mut self) {
        info!("Starting app");
        let controller = self.controller.clone();
        let task = Interval::new(Instant::now(), Duration::from_secs(10))
            .for_each(move |instant| {
                info!("Requesting stats");
                request_stats(controller.clone());
                Ok(())
            })
            .map_err(|e| panic!("interval errored; err={:?}", e));
        tokio::spawn(task);
    }
}
