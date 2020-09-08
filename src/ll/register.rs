use core::fmt::Debug;

/// General error enum for working with registers
#[derive(Debug)]
pub enum RegisterError<IE: Debug> {
    InvalidValue,
    HardwareError(IE),
}

impl<IE: Debug> From<IE> for RegisterError<IE> {
    fn from(value: IE) -> Self {
        RegisterError::HardwareError(value)
    }
}

/// Trait for reading and writing registers
pub trait RegisterInterface {
    /// The type representation of the address
    type Address;
    /// The type representation of the errors the interface can give
    type InterfaceError: Debug;

    /// Reads the register at the given address and puts the data in the value parameter
    fn read_register(
        &mut self,
        address: Self::Address,
        value: &mut [u8],
    ) -> Result<(), Self::InterfaceError>;

    /// Writes the value to the register at the given address
    fn write_register(
        &mut self,
        address: Self::Address,
        value: &[u8],
    ) -> Result<(), Self::InterfaceError>;
}

#[macro_export]
macro_rules! implement_registers {
    (
        $device_name:ident.$register_set_name:ident<$register_address_type:ty> = {
            $(
                $register_name:ident($register_access_specifier:tt, $register_address:expr, $register_size:expr) = {

                }
            ),*
        }
    ) => {
        pub mod $register_set_name {
            use super::*;
            use device_driver::ll::register::RegisterInterface;
            use device_driver::ll::LowLevelDevice;
            use device_driver::implement_reg_accessor;

            impl<'a, I> $device_name<I>
            where
                I: 'a + RegisterInterface<Address = $register_address_type>,
            {
                pub fn $register_set_name(&'a mut self) -> RegisterSet<'a, I> {
                    RegisterSet::new(&mut self.interface)
                }
            }

            /// A struct that borrows the interface from the device.
            /// It implements the read and/or write functionality for the registers.
            pub struct RegAccessor<'a, I, R, W>
            where
                I: 'a + RegisterInterface<Address = $register_address_type>,
            {
                interface: &'a mut I,
                phantom: core::marker::PhantomData<(R, W)>,
            }

            impl<'a, I, R, W> RegAccessor<'a, I, R, W>
            where
                I: 'a + RegisterInterface<Address = $register_address_type>,
            {
                fn new(interface: &'a mut I) -> Self {
                    Self {
                        interface,
                        phantom: Default::default(),
                    }
                }
            }

            /// A struct containing all the register definitions
            pub struct RegisterSet<'a, I>
            where
                I: 'a + RegisterInterface<Address = $register_address_type>,
            {
                interface: &'a mut I,
            }

            impl<'a, I> RegisterSet<'a, I>
            where
                I: 'a + RegisterInterface<Address = $register_address_type>,
            {
                fn new(interface: &'a mut I) -> Self {
                    Self { interface }
                }

                $(
                    pub fn $register_name(&'a mut self) -> RegAccessor<'a, I, $register_name::R, $register_name::W> {
                        RegAccessor::new(&mut self.interface)
                    }
                )*
            }

            $(
                pub mod $register_name {
                    use super::*;

                    pub struct R([u8; $register_size]);
                    pub struct W([u8; $register_size]);

                    impl<'a, I> RegAccessor<'a, I, R, W>
                    where
                        I: RegisterInterface<Address = $register_address_type>,
                    {
                        implement_reg_accessor!($register_access_specifier, $register_address);
                    }

                    impl R {
                        fn zero() -> Self {
                            Self([0; $register_size])
                        }
                    }
                    impl W {
                        fn zero() -> Self {
                            Self([0; $register_size])
                        }
                    }
                }
            )*
        }
    };
}

#[macro_export]
macro_rules! implement_reg_accessor {
    (RO, $address:expr) => {
        /// Reads the register
        pub fn read(&mut self) -> Result<R, RegisterError<I::InterfaceError>> {
            let mut r = R::zero();
            self.interface.read_register($address, &mut r.0)?;
            Ok(r)
        }
    };
    (WO, $address:expr) => {
        /// Writes the value returned by the closure to the register
        pub fn write<F>(&mut self, f: F) -> Result<(), RegisterError<I::InterfaceError>>
        where
            F: FnOnce(W) -> W,
        {
            let w = f(W::zero());
            self.interface.write_register($address, &w.0)?;
            Ok(())
        }
    };
    (RW, $address:expr) => {
        implement_reg_accessor!(RO, $address);
        implement_reg_accessor!(WO, $address);

        /// Reads the register, gives the value to the closure and writes back the value returned by the closure
        pub fn modify<F>(&mut self, f: F) -> Result<(), RegisterError<I::InterfaceError>>
        where
            F: FnOnce(R, W) -> W,
        {
            let r = self.read()?;
            let w = W(r.0.clone());

            let w = f(r, w);

            self.write(|_| w)?;
            Ok(())
        }
    };
}
