/// Ubiquiti UDP discovery port.
pub const PORT: u16 = 10001;

/// Version-1 probe: version 1, command 0, zero length.
pub const V1: [u8; 4] = [0x01, 0x00, 0x00, 0x00];

/// Version-2 probe used by newer firmwares.
pub const V2: [u8; 4] = [0x02, 0x06, 0x00, 0x00];

pub const ALL: [&[u8]; 2] = [&V1, &V2];
