// see https://github.com/torvalds/linux/blob/master/include/uapi/linux/fanotify.h

// TODO add documentation
// TODO check that all the types are correct

/// for fanotify_init
pub mod init {
    /// Flags
    pub mod flag {
        pub const FAN_CLOEXEC: u32 = 0x00000001;
        pub const FAN_NONBLOCK: u32 = 0x00000002;
        pub const FAN_UNLIMITED_QUEUE: u32 = 0x00000010;
        pub const FAN_UNLIMITED_MARKS: u32 = 0x00000020;
        pub const FAN_REPORT_TID: u32 = 0x00000100;
        pub const FAN_REPORT_FID: u32 = 0x00000200;
        pub const FAN_REPORT_DIR_FID: u32 = 0x00000400;
        pub const FAN_REPORT_NAME: u32 = 0x00000800;
    }
    
    /// NotificationClass < Flags
    pub mod notification_class {
        pub const FAN_CLASS_NOTIF: u32 = 0x00000000;
        pub const FAN_CLASS_CONTENT: u32 = 0x00000004;
        pub const FAN_CLASS_PRE_CONTENT: u32 = 0x00000008;
    }
}

/// for fanotify_mark
pub mod mark {
    /// MarkAction < CombinedMarkFlags
    pub mod action {
        pub const FAN_MARK_ADD: u32 = 0x00000001;
        pub const FAN_MARK_REMOVE: u32 = 0x00000002;
        pub const FAN_MARK_FLUSH: u32 = 0x00000080;
    }
    
    /// MarkWhat < CombinedMarkFlags
    pub mod what {
        pub const FAN_MARK_INODE: u32 = 0x00000000;
        pub const FAN_MARK_MOUNT: u32 = 0x00000010;
        pub const FAN_MARK_FILESYSTEM: u32 = 0x00000100;
    }
    
    /// MarkFlags < CombinedMarkFlags
    pub mod flag {
        pub const FAN_MARK_DONT_FOLLOW: u32 = 0x00000004;
        pub const FAN_MARK_ONLYDIR: u32 = 0x00000008;
        pub const FAN_MARK_IGNORED_MASK: u32 = 0x00000020;
        pub const FAN_MARK_IGNORED_SURV_MODIFY: u32 = 0x00000040;
    }
    
    /// mark::Mask
    pub mod mask {
        pub const FAN_ACCESS: u64 = 0x00000001;
        pub const FAN_MODIFY: u64 = 0x00000002;
        pub const FAN_ATTRIB: u64 = 0x00000004;
        pub const FAN_CLOSE_WRITE: u64 = 0x00000008;
        pub const FAN_CLOSE_NOWRITE: u64 = 0x00000010;
        pub const FAN_OPEN: u64 = 0x00000020;
        pub const FAN_MOVED_FROM: u64 = 0x00000040;
        pub const FAN_MOVED_TO: u64 = 0x00000080;
        pub const FAN_CREATE: u64 = 0x00000100;
        pub const FAN_DELETE: u64 = 0x00000200;
        pub const FAN_DELETE_SELF: u64 = 0x00000400;
        pub const FAN_MOVE_SELF: u64 = 0x00000800;
        pub const FAN_OPEN_EXEC: u64 = 0x00001000;
        pub const FAN_Q_OVERFLOW: u64 = 0x00004000;
        pub const FAN_OPEN_PERM: u64 = 0x00010000;
        pub const FAN_ACCESS_PERM: u64 = 0x00020000;
        pub const FAN_OPEN_EXEC_PERM: u64 = 0x00040000;
        pub const FAN_EVENT_ON_CHILD: u64 = 0x08000000;
        pub const FAN_ONDIR: u64 = 0x40000000;
    }
}

pub mod read {
    #[allow(non_camel_case_types)]
    #[derive(Debug, Default)] // TODO remove Debug, Default
    #[repr(C)]
    pub struct fanotify_event_metadata {
        pub event_len: u32,
        pub vers: u8,
        pub reserved: u8,
        pub metadata_len: u16,
        pub mask: u64,
        pub fd: i32,
        pub pid: i32,
    }
    
    #[allow(non_camel_case_types)]
    #[repr(C)]
    pub struct fanotify_event_info_header {
        pub info_type: u8,
        pub pad: u8,
        pub len: u16,
    }
    
    #[allow(non_camel_case_types)]
    #[repr(C)]
    pub struct fanotify_event_file_handle {
        // TODO am I doing the variable sized type from C right here?
        // I know it's not zero-sized at least
        opaque: [libc::c_char; 1], // C VLA
    }
    
    #[allow(non_camel_case_types)]
    #[repr(C)]
    pub struct fanotify_event_info_fid {
        pub hdr: fanotify_event_info_header,
        pub fsid: libc::fsid_t,
        pub handle: fanotify_event_file_handle,
    }
    
    pub const FANOTIFY_METADATA_VERSION: u8 = 3;
    
    pub const FAN_NOFD: i32 = -1;
    
    pub const FAN_EVENT_INFO_TYPE_FID: u8 = 1;
    pub const FAN_EVENT_INFO_TYPE_DFID_NAME: u8 = 2;
    pub const FAN_EVENT_INFO_TYPE_DFID: u8 = 3;
}

pub mod write {
    #[allow(non_camel_case_types)]
    #[repr(C)]
    pub struct fanotify_response {
        pub fd: i32,
        pub response: u32,
    }
    
    pub const FAN_ALLOW: u32 = 0x01;
    pub const FAN_DENY: u32 = 0x02;
    pub const FAN_AUDIT: u32 = 0x10;
}
