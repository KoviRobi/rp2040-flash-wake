*Not yet figured it out fully how to resume this state.* I need to connect a
logic analyzer to the flash pins, but for now I only have remote access to the
RP2040.

Sometimes I found that after several rounds of re-flashing, the RP2040 can go
into a state where it keeps going to the USB bootloader, and writing an UF2
image there still resulted in a reboot to bootloader. After some investigation
this turned out to be because the flash wasn't responsive.

I suspect this is because the flash has powered down for some reason, but I
have not verified this.

This firmware is loaded to RAM via the USB bootloader, thereby skipping the
read from flash step, and then resurrects the flash. It allows you to:

- Send the release power-down command over the (Q)SPI pins of the flash
- Resetting the XIP circuitry via the power state-machine

This is also an example of a RAM only image for the RP2040
