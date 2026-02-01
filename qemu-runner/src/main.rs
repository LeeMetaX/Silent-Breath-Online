use std::path::PathBuf;
use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let kernel_elf = PathBuf::from(args.get(1).expect(
        "Usage: qemu-runner <KERNEL_ELF> <UEFI_BOOTLOADER_EFI> [--no-run]"
    ));
    let bootloader_efi = PathBuf::from(args.get(2).expect(
        "Usage: qemu-runner <KERNEL_ELF> <UEFI_BOOTLOADER_EFI> [--no-run]"
    ));
    let no_run = args.iter().any(|a| a == "--no-run");

    assert!(kernel_elf.exists(), "Kernel ELF not found: {}", kernel_elf.display());
    assert!(bootloader_efi.exists(), "Bootloader EFI not found: {}", bootloader_efi.display());

    let disk_image = kernel_elf.with_extension("img");

    println!("[*] Creating UEFI disk image: {}", disk_image.display());

    create_uefi_disk_image(&disk_image, &bootloader_efi, &kernel_elf);

    println!("[+] Disk image created: {}", disk_image.display());

    if no_run {
        return;
    }

    println!("[*] Launching QEMU with UEFI firmware...");
    println!("    Serial output on stdio. Press Ctrl-A X to exit.\n");

    let ovmf = find_ovmf();

    let status = Command::new("qemu-system-x86_64")
        .args([
            "-bios", &ovmf,
            "-drive", &format!("format=raw,file={}", disk_image.display()),
            "-cpu", "Skylake-Client-v3,+avx2,+aes,+bmi1,+bmi2,+rdrand,+rdseed,+xsave,+xsaveopt",
            "-m", "256M",
            "-serial", "stdio",
            "-display", "none",
            "-no-reboot",
            "-no-shutdown",
        ])
        .status()
        .expect("Failed to launch qemu-system-x86_64");

    std::process::exit(status.code().unwrap_or(1));
}

fn find_ovmf() -> String {
    for path in [
        "/usr/share/ovmf/OVMF.fd",
        "/usr/share/qemu/OVMF.fd",
        "/usr/share/OVMF/OVMF_CODE_4M.fd",
        "/usr/share/edk2/ovmf/OVMF_CODE.fd",
    ] {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    panic!("OVMF firmware not found. Install with: apt install ovmf");
}

fn create_uefi_disk_image(
    disk_path: &std::path::Path,
    bootloader_efi: &std::path::Path,
    kernel_elf: &std::path::Path,
) {
    let bootloader_size = std::fs::metadata(bootloader_efi).expect("stat bootloader").len();
    let kernel_size = std::fs::metadata(kernel_elf).expect("stat kernel").len();

    // 64MB minimum for GPT + FAT32
    let disk_size: u64 = std::cmp::max(
        64 * 1024 * 1024,
        align_up(bootloader_size + kernel_size + 8 * 1024 * 1024, 1024 * 1024),
    );

    let sector_size: u64 = 512;
    let total_sectors = disk_size / sector_size;

    // Partition layout:
    //   LBA 0:          Protective MBR
    //   LBA 1:          GPT Header
    //   LBA 2..33:      GPT Partition Entries (128 entries * 128 bytes = 32 sectors)
    //   LBA 34..N:      ESP (FAT32)
    //   LBA N+1..end-34: (unused)
    //   LBA end-33..end-1: Backup GPT entries
    //   LBA end:        Backup GPT header
    let partition_entries_lba: u64 = 2;
    let partition_table_sectors: u64 = 32;
    let first_usable_lba = partition_entries_lba + partition_table_sectors; // LBA 34
    let last_usable_lba = total_sectors - 1 - 1 - partition_table_sectors;

    let esp_start_lba = first_usable_lba;
    let esp_end_lba = last_usable_lba;
    let esp_size = (esp_end_lba - esp_start_lba + 1) * sector_size;

    // Create FAT32 filesystem for ESP
    let fat_path = disk_path.with_extension("fat.tmp");
    std::fs::write(&fat_path, vec![0u8; esp_size as usize]).expect("create FAT image");

    run_cmd("mkfs.fat", &["-F", "32", "-n", "SILENTBOOT", fat_path.to_str().unwrap()]);
    run_cmd("mmd", &["-i", fat_path.to_str().unwrap(), "::EFI"]);
    run_cmd("mmd", &["-i", fat_path.to_str().unwrap(), "::EFI/BOOT"]);
    run_cmd("mcopy", &[
        "-i", fat_path.to_str().unwrap(),
        bootloader_efi.to_str().unwrap(),
        "::EFI/BOOT/BOOTX64.EFI",
    ]);
    // bootloader_api v0.11 expects kernel at /kernel-x86_64
    run_cmd("mcopy", &[
        "-i", fat_path.to_str().unwrap(),
        kernel_elf.to_str().unwrap(),
        "::kernel-x86_64",
    ]);

    let fat_data = std::fs::read(&fat_path).expect("read FAT image");
    let _ = std::fs::remove_file(&fat_path);

    // Build GPT disk
    let mut disk = vec![0u8; disk_size as usize];

    // Write ESP data at correct offset
    let esp_offset = (esp_start_lba * sector_size) as usize;
    disk[esp_offset..esp_offset + fat_data.len()].copy_from_slice(&fat_data);

    // Protective MBR
    disk[510] = 0x55;
    disk[511] = 0xAA;
    disk[446] = 0x00;       // status
    disk[447] = 0x00;       // CHS first
    disk[448] = 0x02;
    disk[449] = 0x00;
    disk[450] = 0xEE;       // GPT protective type
    disk[451] = 0xFF;       // CHS last
    disk[452] = 0xFF;
    disk[453] = 0xFF;
    disk[454..458].copy_from_slice(&1u32.to_le_bytes());
    let mbr_sectors = (total_sectors - 1).min(0xFFFFFFFF) as u32;
    disk[458..462].copy_from_slice(&mbr_sectors.to_le_bytes());

    // GPT Partition Entry for ESP at LBA 2
    let entry_offset = (partition_entries_lba * sector_size) as usize;
    // EFI System Partition type GUID: C12A7328-F81F-11D2-BA4B-00A0C93EC93B (mixed endian)
    let esp_type_guid: [u8; 16] = [
        0x28, 0x73, 0x2A, 0xC1, 0x1F, 0xF8, 0xD2, 0x11,
        0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B,
    ];
    disk[entry_offset..entry_offset + 16].copy_from_slice(&esp_type_guid);
    // Unique partition GUID
    let unique_guid: [u8; 16] = [
        0x53, 0x42, 0x4F, 0x4E, 0x4C, 0x49, 0x4E, 0x45,
        0x42, 0x52, 0x45, 0x41, 0x54, 0x48, 0x30, 0x31,
    ];
    disk[entry_offset + 16..entry_offset + 32].copy_from_slice(&unique_guid);
    disk[entry_offset + 32..entry_offset + 40].copy_from_slice(&esp_start_lba.to_le_bytes());
    disk[entry_offset + 40..entry_offset + 48].copy_from_slice(&esp_end_lba.to_le_bytes());
    disk[entry_offset + 48..entry_offset + 56].copy_from_slice(&0u64.to_le_bytes());
    // Name: "EFI System" in UTF-16LE
    for (i, c) in "EFI System".encode_utf16().enumerate() {
        let off = entry_offset + 56 + i * 2;
        disk[off..off + 2].copy_from_slice(&c.to_le_bytes());
    }

    // CRC32 of all 128 partition entries (128 * 128 = 16384 bytes)
    let entries_size = 128 * 128;
    let entries_crc = crc32(&disk[entry_offset..entry_offset + entries_size]);

    // Disk GUID
    let disk_guid: [u8; 16] = [
        0x53, 0x49, 0x4C, 0x45, 0x4E, 0x54, 0x42, 0x52,
        0x45, 0x41, 0x54, 0x48, 0x4D, 0x4D, 0x49, 0x4F,
    ];

    // Write primary GPT header at LBA 1
    write_gpt_header(
        &mut disk[sector_size as usize..(sector_size + 92) as usize],
        1,                    // my LBA
        total_sectors - 1,    // alternate LBA
        first_usable_lba,
        last_usable_lba,
        &disk_guid,
        partition_entries_lba,
        128,
        128,
        entries_crc,
    );

    // Write backup partition entries near end of disk
    let backup_entries_lba = total_sectors - 1 - partition_table_sectors;
    let backup_entries_offset = (backup_entries_lba * sector_size) as usize;
    disk.copy_within(entry_offset..entry_offset + entries_size, backup_entries_offset);

    // Write backup GPT header at last LBA
    let backup_header_offset = ((total_sectors - 1) * sector_size) as usize;
    write_gpt_header(
        &mut disk[backup_header_offset..backup_header_offset + 92],
        total_sectors - 1,    // my LBA
        1,                    // alternate LBA
        first_usable_lba,
        last_usable_lba,
        &disk_guid,
        backup_entries_lba,
        128,
        128,
        entries_crc,
    );

    std::fs::write(disk_path, &disk).expect("write disk image");
}

fn write_gpt_header(
    header: &mut [u8],
    my_lba: u64,
    alternate_lba: u64,
    first_usable: u64,
    last_usable: u64,
    disk_guid: &[u8; 16],
    entries_lba: u64,
    num_entries: u32,
    entry_size: u32,
    entries_crc: u32,
) {
    header[0..8].copy_from_slice(b"EFI PART");
    header[8..12].copy_from_slice(&0x00010000u32.to_le_bytes());  // revision 1.0
    header[12..16].copy_from_slice(&92u32.to_le_bytes());         // header size
    header[16..20].copy_from_slice(&0u32.to_le_bytes());          // CRC (fill later)
    header[20..24].copy_from_slice(&0u32.to_le_bytes());          // reserved
    header[24..32].copy_from_slice(&my_lba.to_le_bytes());
    header[32..40].copy_from_slice(&alternate_lba.to_le_bytes());
    header[40..48].copy_from_slice(&first_usable.to_le_bytes());
    header[48..56].copy_from_slice(&last_usable.to_le_bytes());
    header[56..72].copy_from_slice(disk_guid);
    header[72..80].copy_from_slice(&entries_lba.to_le_bytes());
    header[80..84].copy_from_slice(&num_entries.to_le_bytes());
    header[84..88].copy_from_slice(&entry_size.to_le_bytes());
    header[88..92].copy_from_slice(&entries_crc.to_le_bytes());

    let header_crc = crc32(&header[0..92]);
    header[16..20].copy_from_slice(&header_crc.to_le_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

fn align_up(value: u64, align: u64) -> u64 {
    (value + align - 1) & !(align - 1)
}

fn run_cmd(cmd: &str, args: &[&str]) {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!(
            "Failed to run {}: {}. Install with: apt install mtools dosfstools", cmd, e
        ));
    if !output.status.success() {
        panic!("{} {:?} failed: {}", cmd, args, String::from_utf8_lossy(&output.stderr));
    }
}
