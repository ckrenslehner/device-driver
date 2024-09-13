use device_driver::RegisterInterface;

pub struct DeviceInterface {
    device_memory: [u8; 128],
}

impl DeviceInterface {
    pub const fn new() -> Self {
        Self {
            device_memory: [0; 128],
        }
    }
}

impl RegisterInterface for DeviceInterface {
    type Error = ();
    type AddressType = u8;

    fn write_register(
        &mut self,
        address: Self::AddressType,
        data: &[u8],
    ) -> Result<(), Self::Error> {
        self.device_memory[address as usize..][..data.len()].copy_from_slice(data);

        Ok(())
    }

    fn read_register(
        &mut self,
        address: Self::AddressType,
        data: &mut [u8],
    ) -> Result<(), Self::Error> {
        data.copy_from_slice(&self.device_memory[address as usize..][..data.len()]);
        Ok(())
    }
}

pub mod registers {
    use super::*;

    device_driver_macros::implement_device!(
        device_name: MyTestDevice,
        dsl: {
            config {
                type RegisterAddressType = u8;
                type DefaultByteOrder = LE;
            }
            /// This is the Foo register
            register Foo {
                const ADDRESS = 0;
                const SIZE_BITS = 24;

                /// This is a bool!
                value0: bool = 0..1,
                value1: uint = 1..16,
                value2: int = 16..24,
            }
        }
    );

    #[test]
    fn test_basic_read_modify_write() {
        let mut device = MyTestDevice::new(DeviceInterface::new());

        device.foo().write(|w| w.set_value_1(12345)).unwrap();
        let reg = device.foo().read().unwrap();

        assert_eq!(reg.value_0(), false);
        assert_eq!(reg.value_1(), 12345u16);
        assert_eq!(reg.value_2(), 0i8);

        device
            .foo()
            .modify(|w| w.set_value_0(true).set_value_2(-1))
            .unwrap();

        let reg = device.foo().read().unwrap();

        assert_eq!(reg.value_0(), true);
        assert_eq!(reg.value_1(), 12345u16);
        assert_eq!(reg.value_2(), -1);

        // device.interface.device_memory
    }
}
