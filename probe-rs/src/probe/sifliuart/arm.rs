use crate::probe::sifliuart::arm::memory_ap::MemoryAp;
use crate::architecture::arm::communication_interface::{DapProbe, SwdSequence, UninitializedArmProbe};
use crate::architecture::arm::dp::{DpAddress, DpRegisterAddress};
use crate::architecture::arm::memory::{ArmMemoryInterface};
use crate::architecture::arm::sequences::ArmDebugSequence;
use crate::architecture::arm::{ap, ArmError, ArmProbeInterface, DapAccess, FullyQualifiedApAddress, SwoAccess, SwoConfig};
use crate::probe::sifliuart::{SifliUart, SifliUartCommand, SifliUartResponse};
use crate::probe::{DebugProbeError, Probe};
use crate::Error as ProbeRsError;
use crate::MemoryInterface;
use std::cmp::{max, min};
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;
use zerocopy::IntoBytes;
use crate::architecture::arm::ap::{memory_ap, ApRegister, CSW, IDR, CFG, AccessPortType, MemoryApType};

#[derive(Debug)]
pub(crate) struct UninitializedSifliUartArmProbe {
    pub(crate) probe: Box<SifliUart>,
}

#[derive(Debug)]
pub(crate) struct SifliUartArmDebug {
    probe: Box<SifliUart>,

    /// Information about the APs of the target.
    /// APs are identified by a number, starting from zero.
    pub access_ports: BTreeSet<FullyQualifiedApAddress>,

    /// A copy of the sequence that was passed during initialization
    sequence: Arc<dyn ArmDebugSequence>,
}

impl SifliUartArmDebug {
    fn new(
        probe: Box<SifliUart>,
        sequence: Arc<dyn ArmDebugSequence>,
    ) -> Result<Self, (Box<dyn UninitializedArmProbe>, ArmError)> {
        // Determine the number and type of available APs.
        let interface = Self {
            probe,
            access_ports: BTreeSet::new(),
            sequence,
        };

        Ok(interface)
    }
}

impl UninitializedSifliUartArmProbe {
    fn new(probe: Box<SifliUart>) -> Self {
        Self { probe }
    }
}

#[allow(unused)]
impl SwdSequence for UninitializedSifliUartArmProbe {
    fn swj_sequence(&mut self, bit_len: u8, bits: u64) -> Result<(), DebugProbeError> {
        Err(DebugProbeError::NotImplemented {
            function_name: "swj_sequence",
        })
    }

    fn swj_pins(
        &mut self,
        pin_out: u32,
        pin_select: u32,
        pin_wait: u32,
    ) -> Result<u32, DebugProbeError> {
        Err(DebugProbeError::NotImplemented {
            function_name: "swj_pins",
        })
    }
}

impl UninitializedArmProbe for UninitializedSifliUartArmProbe {
    #[tracing::instrument(level = "trace", skip(self, sequence))]
    fn initialize(
        self: Box<Self>,
        sequence: Arc<dyn ArmDebugSequence>,
        dp: DpAddress,
    ) -> Result<
        Box<(dyn ArmProbeInterface + 'static)>,
        (Box<(dyn UninitializedArmProbe + 'static)>, ProbeRsError),
    > {
        assert_eq!(dp, DpAddress::Default, "Multidrop not supported on Sifli");

        let interface = SifliUartArmDebug::new(self.probe, sequence)
            .map_err(|(s, e)| (s as Box<_>, crate::error::Error::from(e)))?;

        Ok(Box::new(interface))
    }

    fn close(self: Box<Self>) -> Probe {
        Probe::from_attached_probe(self.probe)
    }
}

#[allow(unused)]
impl DapAccess for SifliUartArmDebug {
    fn read_raw_dp_register(
        &mut self,
        dp: DpAddress,
        addr: DpRegisterAddress,
    ) -> Result<u32, ArmError> {
        Err(ArmError::NotImplemented("dp register read not implemented"))
    }

    fn write_raw_dp_register(
        &mut self,
        dp: DpAddress,
        addr: DpRegisterAddress,
        value: u32,
    ) -> Result<(), ArmError> {
        Ok(())
    }

    fn read_raw_ap_register(
        &mut self,
        ap: &FullyQualifiedApAddress,
        addr: u64,
    ) -> Result<u32, ArmError> {
        // 伪造一个MEM-AP的IDR寄存器
        if addr == IDR::ADDRESS {
            let idr = 0x24770031;
            return Ok(idr);
        } else if addr == CSW::ADDRESS {
            return Ok(0x23000052)
        }
        else if addr == CFG::ADDRESS {
            return Ok(0x00000000)
        }
        Err(ArmError::NotImplemented("ap register read not implemented"))
    }

    fn write_raw_ap_register(
        &mut self,
        ap: &FullyQualifiedApAddress,
        addr: u64,
        value: u32,
    ) -> Result<(), ArmError> {
        Ok(())
    }

    fn try_dap_probe(&self) -> Option<&dyn DapProbe> {
        None
    }
}

#[allow(unused)]
impl SwdSequence for SifliUartArmDebug {
    fn swj_sequence(&mut self, bit_len: u8, bits: u64) -> Result<(), DebugProbeError> {
        Err(DebugProbeError::NotImplemented {
            function_name: "swj_sequence",
        })
    }

    fn swj_pins(
        &mut self,
        pin_out: u32,
        pin_select: u32,
        pin_wait: u32,
    ) -> Result<u32, DebugProbeError> {
        Err(DebugProbeError::NotImplemented {
            function_name: "swj_pins",
        })
    }
}

impl SwoAccess for SifliUartArmDebug {
    fn enable_swo(&mut self, _config: &SwoConfig) -> Result<(), ArmError> {
        Err(ArmError::NotImplemented("swo not implemented"))
    }

    fn disable_swo(&mut self) -> Result<(), ArmError> {
        Err(ArmError::NotImplemented("swo not implemented"))
    }

    fn read_swo_timeout(&mut self, _timeout: Duration) -> Result<Vec<u8>, ArmError> {
        Err(ArmError::NotImplemented("swo not implemented"))
    }
}

impl ArmMemoryInterface for SifliUartMemoryInterface<'_> {
    fn fully_qualified_address(&self) -> FullyQualifiedApAddress {
        self.current_ap.ap_address().clone()
    }

    fn base_address(&mut self) -> Result<u64, ArmError> {
        self.current_ap.base_address(self.probe)
    }

    fn get_swd_sequence(&mut self) -> Result<&mut dyn SwdSequence, DebugProbeError> {
        Ok(self.probe)
    }

    fn get_arm_probe_interface(&mut self) -> Result<&mut dyn ArmProbeInterface, DebugProbeError> {
        Ok(self.probe)
    }

    fn get_dap_access(&mut self) -> Result<&mut dyn DapAccess, DebugProbeError> {
        Ok(self.probe)
    }

    fn generic_status(&mut self) -> Result<CSW, ArmError> {
        Err(ArmError::Probe(DebugProbeError::InterfaceNotAvailable {
            interface_name: "ARM",
        }))
    }
}

#[allow(unused)]
impl ArmProbeInterface for SifliUartArmDebug {
    fn reinitialize(&mut self) -> Result<(), ArmError> {
        Ok(())
    }

    fn access_ports(
        &mut self,
        dp: DpAddress,
    ) -> Result<BTreeSet<FullyQualifiedApAddress>, ArmError> {
        Err(ArmError::NotImplemented("access_ports not implemented"))
    }

    fn close(self: Box<Self>) -> Probe {
        Probe::from_attached_probe(self.probe)
    }

    fn current_debug_port(&self) -> DpAddress {
        DpAddress::Default
    }

    fn memory_interface(
        &mut self,
        access_port: &FullyQualifiedApAddress,
    ) -> Result<Box<dyn ArmMemoryInterface + '_>, ArmError> {
        let memory_ap = MemoryAp::new(self, access_port)?;
        let interface = SifliUartMemoryInterface {
            probe: self,
            current_ap: memory_ap,
        };

        Ok(Box::new(interface) as _)
    }
}

#[derive(Debug)]
struct SifliUartMemoryInterface<'probe> {
    probe: &'probe mut SifliUartArmDebug,
    current_ap: MemoryAp,
}

impl SifliUartMemoryInterface<'_> {
    fn write(&mut self, address: u64, data: &[u8]) -> Result<(), ArmError> {
        let sifli_uart = &mut self.probe.probe;

        if data.is_empty() {
            return Ok(());
        }

        let addr_usize = address as usize;
        // 计算对齐后的起始地址和结束地址
        let start_aligned = addr_usize - (addr_usize % 4);
        let end_aligned = ((addr_usize + data.len() + 3) / 4) * 4;
        let total_bytes = end_aligned - start_aligned;
        let total_words = total_bytes / 4;

        // 分配缓冲区，保存整个对齐区域的数据
        let mut buffer = vec![0u8; total_bytes];

        // 新数据写入的区域为 [addr_usize, addr_usize + data.len())
        // 遍历整个对齐区域的每个4字节块
        for i in 0..total_words {
            let block_addr = start_aligned + i * 4;
            let block_end = block_addr + 4;

            // 判断当前 4 字节块是否被写入的新数据“完全覆盖”
            // 如果 block 完全落入新数据区域，则直接拷贝新数据
            if block_addr >= addr_usize && block_end <= addr_usize + data.len() {
                let offset_in_data = block_addr - addr_usize;
                buffer[i * 4..i * 4 + 4].copy_from_slice(&data[offset_in_data..offset_in_data + 4]);
            } else {
                // 其余情况（头部或尾部不完全覆盖）：
                // 先调用 MEMRead 读出原有的 4 字节数据块
                let resp = sifli_uart
                    .command(SifliUartCommand::MEMRead {
                        addr: block_addr as u32,
                        len: 1,
                    })
                    .map_err(|e| ArmError::Other(format!("{:?}", e)))?;
                let mut block: [u8; 4] = match resp {
                    SifliUartResponse::MEMRead { data: d } if d.len() == 4 => {
                        [d[0], d[1], d[2], d[3]]
                    }
                    _ => return Err(ArmError::Other("MEMRead Error".to_string())),
                };
                // 计算该块与新数据区域的重叠部分
                let overlap_start = max(block_addr, addr_usize);
                let overlap_end = min(block_end, addr_usize + data.len());
                if overlap_start < overlap_end {
                    let in_block_offset = overlap_start - block_addr;
                    let in_data_offset = overlap_start - addr_usize;
                    let overlap_len = overlap_end - overlap_start;
                    block[in_block_offset..in_block_offset + overlap_len]
                        .copy_from_slice(&data[in_data_offset..in_data_offset + overlap_len]);
                }
                buffer[i * 4..i * 4 + 4].copy_from_slice(&block);
            }
        }

        let words: Vec<u32> = buffer
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes(chunk.try_into().expect("chunk length is 4")))
            .collect();

        // 一次性写入整个对齐区域
        sifli_uart
            .command(SifliUartCommand::MEMWrite {
                addr: start_aligned as u32,
                data: &words,
            })
            .map_err(|e| ArmError::Other(format!("{:?}", e)))?;

        Ok(())
    }

    fn read(&mut self, address: u64, data: &mut [u8]) -> Result<(), ArmError> {
        let sifli_uart = &mut self.probe.probe;

        // 若数据长度为0，直接返回
        if data.is_empty() {
            return Ok(());
        }

        let addr = address as usize;
        let end_addr = addr + data.len();

        // 计算对齐区域：[start_aligned, end_aligned)
        let start_aligned = addr - (addr % 4);
        let end_aligned = ((end_addr + 3) / 4) * 4;
        let total_bytes = end_aligned - start_aligned;
        let total_words = total_bytes / 4;

        // 一次性读取对齐区域内所有数据
        let resp = sifli_uart
            .command(SifliUartCommand::MEMRead {
                addr: start_aligned as u32,
                len: total_words as u16,
            })
            .map_err(|e| ArmError::Other(format!("{:?}", e)))?;

        let buf = match resp {
            SifliUartResponse::MEMRead { data } if data.len() == total_bytes => data,
            _ => return Err(ArmError::Other("MEMRead Error".to_string())),
        };

        // 将关心的区域数据拷贝至 data
        let offset = addr - start_aligned;
        data.copy_from_slice(&buf[offset..offset + data.len()]);

        Ok(())
    }
}

#[allow(unused)]
impl MemoryInterface<ArmError> for SifliUartMemoryInterface<'_> {
    fn supports_native_64bit_access(&mut self) -> bool {
        true
    }

    fn read_64(&mut self, address: u64, data: &mut [u64]) -> Result<(), ArmError> {
        self.read(address, data.as_mut_bytes())
    }

    fn read_32(&mut self, address: u64, data: &mut [u32]) -> Result<(), ArmError> {
        self.read(address, data.as_mut_bytes())
    }

    fn read_16(&mut self, address: u64, data: &mut [u16]) -> Result<(), ArmError> {
        self.read(address, data.as_mut_bytes())
    }

    fn read_8(&mut self, address: u64, data: &mut [u8]) -> Result<(), ArmError> {
        self.read(address, data)
    }

    fn write_64(&mut self, address: u64, data: &[u64]) -> Result<(), ArmError> {
        self.write(address, data.as_bytes())
    }

    fn write_32(&mut self, address: u64, data: &[u32]) -> Result<(), ArmError> {
        self.write(address, data.as_bytes())
    }

    fn write_16(&mut self, address: u64, data: &[u16]) -> Result<(), ArmError> {
        self.write(address, data.as_bytes())
    }

    fn write_8(&mut self, address: u64, data: &[u8]) -> Result<(), ArmError> {
        self.write(address, data)
    }

    fn supports_8bit_transfers(&self) -> Result<bool, ArmError> {
        Ok(true)
    }

    fn flush(&mut self) -> Result<(), ArmError> {
        Ok(())
    }
}
