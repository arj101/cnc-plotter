echo "Running cargo objcopy..."
cargo objcopy --release --features cortex-m-semihosting/no-semihosting -- -O binary cnc-plotter.bin
echo "Flashing..."
st-flash write cnc-plotter.bin 0x8000000