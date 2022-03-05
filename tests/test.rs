use device_driver::device_driver;

#[device_driver(options = [option1, option2])]
mod sr1000 {
    #[field(infallible, bits = 1)] // Checked at compile time (will result in a mem::transmute)
    pub enum Bit {
        Clear = 0,
        Set = 1,
    }

    /// The mode of the pin. 2 bits.
    #[field(infallible, bits = 2)] // Checked at compile time (will result in a mem::transmute)
    pub enum SleepMode {
        IdleSleep = 0b00,
        ShallowSleep = 0b01,
        DeepSleep = 0b10,
        Shutdown = 0b11,
    }

    #[field(fallible, bits = 2)] // Checked at run time (will result in a try_into)
    pub enum TxMode {
        Direct = 0b00,
        Delayed = 0b01,
        AutoRx = 0b10,
    }

    /// The global register set
    #[registers(maybe_some_settings)]
    mod registers {
        #[register(rw, address = 0x0, size = 1)]
        struct Status {
            /// Docs, single bit
            #[at(7)]
            stat2irq: Bit,

            /// Derived infallible
            #[at(5..7)]
            slpdepth: SleepDepth,

            /// Derived fallible
            #[at(3..5)]
            txmode: TxMode,
        }
    }
}

#[test]
fn test() {
    println!("testing testing");
}
