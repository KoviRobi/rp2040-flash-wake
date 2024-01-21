MEMORY {
  /* NOTE K = KiB = 1024 bytes */
  /*
   * For a flash-based image we would use

  BOOT   (rx)  : ORIGIN = 0x10000000, LENGTH = 256
  FLASH  (rx)  : ORIGIN = 0x10000100, LENGTH = 2M - 256
  RAM    (rwx) : ORIGIN = 0x20000000, LENGTH = 264K

   * but we are building an USB bootloader -> copy-to-RAM elf image so we
   * pretend that the FLASH is in the RAM. See
   *
   * - https://github.com/JoNil/elf2uf2-rs/blob/2038e9a199101ee8a16d046a87136be2a607001d/src/main.rs#L75
   * - https://github.com/JoNil/elf2uf2-rs/blob/2038e9a199101ee8a16d046a87136be2a607001d/src/elf.rs#L101
   * - https://github.com/JoNil/elf2uf2-rs/blob/2038e9a199101ee8a16d046a87136be2a607001d/src/address_range.rs#L56
   * - https://github.com/JoNil/elf2uf2-rs/blob/2038e9a199101ee8a16d046a87136be2a607001d/src/address_range.rs#L41
   *
   * - https://github.com/raspberrypi/pico-bootrom/blob/ef22cd8ede5bc007f81d7f2416b48db90f313434/bootrom/virtual_disk.c#L374
   */
  FLASH  (rx)  : ORIGIN = 0x20000000,        LENGTH = 128K
  RAM    (rwx) : ORIGIN = 0x20000000 + 128K, LENGTH = 264K - 128K
}
