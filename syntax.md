```rust
#[device_driver]
mod SR1000 {
    /// The mode of the pin. 2 bits.
    #[generate(infallible, bits = 2)] // Checked at compile time (will result in a mem::transmute)
    pub enum SleepMode {
        IdleSleep = 0b00,
        ShallowSleep = 0b01,
        DeepSleep = 0b10,
        Shutdown = 0b11,
    }

    #[generate(fallible, bits = 2)] // Checked at run time (will result in a try_into)
    pub enum TxMode {
        Direct = 0b00,
        Delayed = 0b01,
        AutoRx = 0b10,
    }

    /// The global register set
    #[registers(maybe_some_settings)]
    mod registers {
        #[register(type = RO, address = 0x0, size = 1)]
        struct Status {
            /// Docs, single bit
            const stat2irq: Bit = 7;

            /// Derived infallible
            const slpdepth: SleepDepth = 5..7;

            /// Derived fallible
            const txmode: TxMode = 3..5;
        }
    }
}
```
