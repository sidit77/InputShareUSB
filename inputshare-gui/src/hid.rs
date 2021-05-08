#![allow(non_upper_case_globals)]

use yawi::{WindowsScanCode, VirtualKey};

pub type HidScanCode = u8;

#[allow(dead_code)]
bitflags::bitflags! {
    pub struct HidModifierKeys: u8 {
        const None    = 0x00;
        const LCtrl   = 0x01;
        const LShift  = 0x02;
        const LAlt    = 0x04;
        const LMeta   = 0x08;
        const RCtrl   = 0x10;
        const RShift  = 0x20;
        const RAlt    = 0x40;
        const RMeta   = 0x80;
    }
}

impl HidModifierKeys {
    pub fn from_virtual_key(key: &VirtualKey) -> Option<Self>{
        match key {
            VirtualKey::LShift   => Some(HidModifierKeys::LShift),
            VirtualKey::LControl => Some(HidModifierKeys::LCtrl),
            VirtualKey::LWin     => Some(HidModifierKeys::LMeta),
            VirtualKey::LMenu    => Some(HidModifierKeys::LAlt),
            VirtualKey::RShift   => Some(HidModifierKeys::RShift),
            VirtualKey::RControl => Some(HidModifierKeys::RCtrl),
            VirtualKey::RWin     => Some(HidModifierKeys::RMeta),
            VirtualKey::RMenu    => Some(HidModifierKeys::RAlt),
            _                    => None
        }
    }

    pub fn to_virtual_keys(&self) -> Vec<VirtualKey> {
        let mut v = Vec::new();
        if self.contains(HidModifierKeys::LShift){ v.push(VirtualKey::LShift); }
        if self.contains(HidModifierKeys::LCtrl ){ v.push(VirtualKey::LControl); }
        if self.contains(HidModifierKeys::LMeta ){ v.push(VirtualKey::LWin); }
        if self.contains(HidModifierKeys::LAlt  ){ v.push(VirtualKey::LMenu); }
        if self.contains(HidModifierKeys::RShift){ v.push(VirtualKey::RShift); }
        if self.contains(HidModifierKeys::RCtrl ){ v.push(VirtualKey::RControl); }
        if self.contains(HidModifierKeys::RMeta ){ v.push(VirtualKey::RWin); }
        if self.contains(HidModifierKeys::RAlt  ){ v.push(VirtualKey::RMenu); }
        v
    }

    pub fn to_byte(&self) -> u8 {
        self.bits
    }
}

#[allow(dead_code)]
bitflags::bitflags! {
    pub struct HidMouseButtons: u8 {
        const None    = 0x00;
        const LButton = 0x01;
        const RButton = 0x02;
        const MButton = 0x04;
        const Button4 = 0x08;
        const Button5 = 0x10;
    }
}

impl HidMouseButtons {
    pub fn from_virtual_key(key: &VirtualKey) -> Option<Self>{
        match key {
            VirtualKey::LButton  => Some(HidMouseButtons::LButton),
            VirtualKey::RButton  => Some(HidMouseButtons::RButton),
            VirtualKey::MButton  => Some(HidMouseButtons::MButton),
            VirtualKey::XButton1 => Some(HidMouseButtons::Button4),
            VirtualKey::XButton2 => Some(HidMouseButtons::Button5),
            _                    => None
        }
    }

    pub fn to_virtual_keys(&self) -> Vec<VirtualKey> {
        let mut v = Vec::new();
        if self.contains(HidMouseButtons::LButton){ v.push(VirtualKey::LButton); }
        if self.contains(HidMouseButtons::RButton){ v.push(VirtualKey::RButton); }
        if self.contains(HidMouseButtons::MButton){ v.push(VirtualKey::MButton); }
        if self.contains(HidMouseButtons::Button4){ v.push(VirtualKey::XButton1); }
        if self.contains(HidMouseButtons::Button5){ v.push(VirtualKey::XButton2); }
        v
    }

    pub fn to_byte(&self) -> u8 {
        self.bits
    }
}


pub fn convert_win2hid(scancode: &WindowsScanCode) -> Option<HidScanCode> {
    match scancode {
         0x1 => Some(0x29),
         0x2 => Some(0x1e),
         0x3 => Some(0x1f),
         0x4 => Some(0x20),
         0x5 => Some(0x21),
         0x6 => Some(0x22),
         0x7 => Some(0x23),
         0x8 => Some(0x24),
         0x9 => Some(0x25),
         0xa => Some(0x26),
         0xb => Some(0x27),
         0xc => Some(0x2d),
         0xd => Some(0x2e),
         0xe => Some(0x2a),
         0xf => Some(0x2b),
         0x10 => Some(0x14),
         0x11 => Some(0x1a),
         0x12 => Some(0x8),
         0x13 => Some(0x15),
         0x14 => Some(0x17),
         0x15 => Some(0x1c),
         0x16 => Some(0x18),
         0x17 => Some(0xc),
         0x18 => Some(0x12),
         0x19 => Some(0x13),
         0x1a => Some(0x2f),
         0x1b => Some(0x30),
         0x1c => Some(0x28),
         0x1d => Some(0xe0),
         0x1e => Some(0x4),
         0x1f => Some(0x16),
         0x20 => Some(0x7),
         0x21 => Some(0x9),
         0x22 => Some(0xa),
         0x23 => Some(0xb),
         0x24 => Some(0xd),
         0x25 => Some(0xe),
         0x26 => Some(0xf),
         0x27 => Some(0x33),
         0x28 => Some(0x34),
         0x29 => Some(0x35),
         0x2a => Some(0xe1),
         0x2b => Some(0x31),
         0x2c => Some(0x1d),
         0x2d => Some(0x1b),
         0x2e => Some(0x6),
         0x2f => Some(0x19),
         0x30 => Some(0x5),
         0x31 => Some(0x11),
         0x32 => Some(0x10),
         0x33 => Some(0x36),
         0x34 => Some(0x37),
         0x35 => Some(0x38),
         0x36 => Some(0xe5),
         0x37 => Some(0x55),
         0x38 => Some(0xe2),
         0x39 => Some(0x2c),
         0x3a => Some(0x39),
         0x3b => Some(0x3a),
         0x3c => Some(0x3b),
         0x3d => Some(0x3c),
         0x3e => Some(0x3d),
         0x3f => Some(0x3e),
         0x40 => Some(0x3f),
         0x41 => Some(0x40),
         0x42 => Some(0x41),
         0x43 => Some(0x42),
         0x44 => Some(0x43),
         0x45 => Some(0x53),
         0x46 => Some(0x47),
         0x47 => Some(0x5f),
         0x48 => Some(0x60),
         0x49 => Some(0x61),
         0x4a => Some(0x56),
         0x4b => Some(0x5c),
         0x4c => Some(0x5d),
         0x4d => Some(0x5e),
         0x4e => Some(0x57),
         0x4f => Some(0x59),
         0x50 => Some(0x5a),
         0x51 => Some(0x5b),
         0x52 => Some(0x62),
         0x53 => Some(0x63),
         0x54 => Some(0x46),
         0x56 => Some(0x64),
         0x57 => Some(0x44),
         0x58 => Some(0x45),
         0x59 => Some(0x67),
         0x5c => Some(0x8c),
         0x64 => Some(0x68),
         0x65 => Some(0x69),
         0x66 => Some(0x6a),
         0x67 => Some(0x6b),
         0x68 => Some(0x6c),
         0x69 => Some(0x6d),
         0x6a => Some(0x6e),
         0x6b => Some(0x6f),
         0x6c => Some(0x70),
         0x6d => Some(0x71),
         0x6e => Some(0x72),
         0x70 => Some(0x88),
         0x73 => Some(0x87),
         0x76 => Some(0x73),
         0x77 => Some(0x93),
         0x78 => Some(0x92),
         0x79 => Some(0x8a),
         0x7b => Some(0x8b),
         0x7d => Some(0x89),
         0x7e => Some(0x85),
         0xf1 => Some(0x91),
         0xf2 => Some(0x90),
         0xfc => Some(0x2),
         0xff => Some(0x1),
         0xe010 => Some(0xea),
         0xe019 => Some(0xeb),
         0xe01c => Some(0x58),
         0xe01d => Some(0xe4),
         0xe020 => Some(0xef),
         0xe021 => Some(0xfb),
         0xe022 => Some(0xe8),
         0xe024 => Some(0xe9),
         0xe02e => Some(0xee),
         0xe030 => Some(0xed),
         0xe032 => Some(0xf0),
         0xe035 => Some(0x54),
         0xe038 => Some(0xe6),
         0xe047 => Some(0x4a),
         0xe048 => Some(0x52),
         0xe049 => Some(0x4b),
         0xe04b => Some(0x50),
         0xe04d => Some(0x4f),
         0xe04f => Some(0x4d),
         0xe050 => Some(0x51),
         0xe051 => Some(0x4e),
         0xe052 => Some(0x49),
         0xe053 => Some(0x4c),
         0xe05b => Some(0xe3),
         0xe05c => Some(0xe7),
         0xe05d => Some(0x65),
         0xe05e => Some(0x66),
         0xe05f => Some(0x82),
         0xe063 => Some(0x83),
         0xe065 => Some(0xf4),
         0xe067 => Some(0xfa),
         0xe068 => Some(0xf3),
         0xe069 => Some(0xf2),
         0xe06a => Some(0xf1),
         0xe06d => Some(0xec),
         0xe11d => Some(0x48),
        _ => None
    }
}