// see https://github.com/torvalds/linux/blob/master/include/uapi/linux/fanotify.h

// TODO add documentation
// TODO check that all the types are correct

/// for fanotify_init
pub mod init {
    /// Flags
    pub mod flag {
        /// Set the close-on-exec flag on the new file descriptor
        pub const FAN_CLOEXEC: u32 = 0x00000001;
        /// Enable the nonblocking flag  for the file descriptor. Reading fd from not block.
        pub const FAN_NONBLOCK: u32 = 0x00000002;
        /// Remove the limit of 16384 events for the event queue. Requires CAP_SYS_ADMIN
        pub const FAN_UNLIMITED_QUEUE: u32 = 0x00000010;
        /// Remove the limit of 8192 marks. Requires CAP_SYS_ADMIN
        pub const FAN_UNLIMITED_MARKS: u32 = 0x00000020;
        /// Report TID instead PID in PID field of the fanotify_event_metadata supplied to read
        pub const FAN_REPORT_TID: u32 = 0x00000100;
        /// Allows the receipt of events which contain additional info about
        /// the underlying filesystem object correlated to an event
        pub const FAN_REPORT_FID: u32 = 0x00000200;
        /// Initialized fanotify groups with this flag will contain additional
        /// info about the directory object correlated to an event
        pub const FAN_REPORT_DIR_FID: u32 = 0x00000400;
        /// Initialized fanotify groups with this flag will contain additional
        /// info about the name of the directory entry correlated to an event
        pub const FAN_REPORT_NAME: u32 = 0x00000800;
    }
    
    /// NotificationClass < Flags
    pub mod notification_class {
        /// Does not need to be specified. Only allows the receipt of events
        /// notifying that a file has been accessed.
        pub const FAN_CLASS_NOTIF: u32 = 0x00000000;
        /// Allows receipt of events notifying that a file has been accessed
        /// and events for permission decisions if a file may be accessed.
        /// Intended for event listeners that need to access files when they
        /// already contain their final content
        pub const FAN_CLASS_CONTENT: u32 = 0x00000004;
        /// Allows receipt of events notifying that a file has been accessed and
        /// events for permission decisions if a file may be accessed.
        /// Intended for event listeners that need to access files before they
        /// contain their final content
        pub const FAN_CLASS_PRE_CONTENT: u32 = 0x00000008;
    }
}

/// for fanotify_mark
pub mod mark {
    /// MarkAction < CombinedMarkFlags
    pub mod action {
        /// Events in mask will be added to the mark mask or ignore mask
        pub const FAN_MARK_ADD: u32 = 0x00000001;
        /// Events in argument mask will be removed from the mark mask or ignore mask
        pub const FAN_MARK_REMOVE: u32 = 0x00000002;
        /// Remove either all marks for filesystems, all marks for mounts,
        /// or all marks for directories and files from the fanotify group
        pub const FAN_MARK_FLUSH: u32 = 0x00000080;
    }
    
    /// MarkWhat < CombinedMarkFlags
    pub mod what {
        /// Does not need to be specified.
        pub const FAN_MARK_INODE: u32 = 0x00000000;
        /// Mark the mount point specified by pathname (or mount point containing pathname)
        pub const FAN_MARK_MOUNT: u32 = 0x00000010;
        /// Mark filesystem specified by pathname
        pub const FAN_MARK_FILESYSTEM: u32 = 0x00000100;
    }
    
    /// MarkFlags < CombinedMarkFlags
    pub mod flag {
        /// If pathname is symbolic link, mark link itself
        pub const FAN_MARK_DONT_FOLLOW: u32 = 0x00000004;
        /// If the filesystem object to be marked is not a directory, ENOTDIR will be raised
        pub const FAN_MARK_ONLYDIR: u32 = 0x00000008;
        /// The events in mask shall be added to or remove from the ignore mask
        pub const FAN_MARK_IGNORED_MASK: u32 = 0x00000020;
        /// The ignore mask shall survive modify events
        pub const FAN_MARK_IGNORED_SURV_MODIFY: u32 = 0x00000040;
    }
    
    /// mark::Mask
    pub mod mask {
        /// Create an event when file or directory is accessed (read)
        pub const FAN_ACCESS: u64 = 0x00000001;
        /// Create an event when a file is modified (write)
        pub const FAN_MODIFY: u64 = 0x00000002;
        /// Create an event when the metadata for a file or directory has changed
        pub const FAN_ATTRIB: u64 = 0x00000004;
        /// Create an event when a writable file is closed
        pub const FAN_CLOSE_WRITE: u64 = 0x00000008;
        /// Create an event when a read-only file is closed
        pub const FAN_CLOSE_NOWRITE: u64 = 0x00000010;
        /// Create an event when a file or directory is opened
        pub const FAN_OPEN: u64 = 0x00000020;
        /// Create an event when a file or directory has been moved from a marked parent directory
        pub const FAN_MOVED_FROM: u64 = 0x00000040;
        /// Create an event when a file or directory has been moved to a marked parent directory
        pub const FAN_MOVED_TO: u64 = 0x00000080;
        /// Create an event when a file or directory has been created in a marked parent directory
        pub const FAN_CREATE: u64 = 0x00000100;
        /// Create an event when a file or directory has been deleted in a marked parent directory
        pub const FAN_DELETE: u64 = 0x00000200;
        /// Create an event when a marked file or directory has been deleted
        pub const FAN_DELETE_SELF: u64 = 0x00000400;
        /// Create an event when a marked file or directory has been moved
        pub const FAN_MOVE_SELF: u64 = 0x00000800;
        /// Create an event when a file is open with intent to execute
        pub const FAN_OPEN_EXEC: u64 = 0x00001000;
        /// Create an event when an overflow of the event queue occurs
        pub const FAN_Q_OVERFLOW: u64 = 0x00004000;
        /// Create an event when a permission to open a file or directory is requested
        pub const FAN_OPEN_PERM: u64 = 0x00010000;
        /// Create an event when a permission to read a file or directory is requested
        pub const FAN_ACCESS_PERM: u64 = 0x00020000;
        /// Create an event when a permission to open a file for execution is requested
        pub const FAN_OPEN_EXEC_PERM: u64 = 0x00040000;
        /// Events for the immediate children of marked directories shall be created
        pub const FAN_EVENT_ON_CHILD: u64 = 0x08000000;
        /// Create events for directories
        pub const FAN_ONDIR: u64 = 0x40000000;
    }
}

pub mod read {
    #[allow(non_camel_case_types)]
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
