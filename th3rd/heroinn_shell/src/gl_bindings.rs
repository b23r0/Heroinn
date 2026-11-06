
        mod __gl_imports {
            pub use std::mem;
            pub use std::os::raw;
        }
    

        #[inline(never)]
        fn metaloadfn(loadfn: &mut dyn FnMut(&'static str) -> *const __gl_imports::raw::c_void,
                      symbol: &'static str,
                      fallbacks: &[&'static str]) -> *const __gl_imports::raw::c_void {
            let mut ptr = loadfn(symbol);
            if ptr.is_null() {
                for &sym in fallbacks {
                    ptr = loadfn(sym);
                    if !ptr.is_null() { break; }
                }
            }
            ptr
        }
    

        pub mod types {
            #![allow(non_camel_case_types, non_snake_case, dead_code, missing_copy_implementations)]
    
// Common types from OpenGL 1.1
pub type GLenum = super::__gl_imports::raw::c_uint;
pub type GLboolean = super::__gl_imports::raw::c_uchar;
pub type GLbitfield = super::__gl_imports::raw::c_uint;
pub type GLvoid = super::__gl_imports::raw::c_void;
pub type GLbyte = super::__gl_imports::raw::c_char;
pub type GLshort = super::__gl_imports::raw::c_short;
pub type GLint = super::__gl_imports::raw::c_int;
pub type GLclampx = super::__gl_imports::raw::c_int;
pub type GLubyte = super::__gl_imports::raw::c_uchar;
pub type GLushort = super::__gl_imports::raw::c_ushort;
pub type GLuint = super::__gl_imports::raw::c_uint;
pub type GLsizei = super::__gl_imports::raw::c_int;
pub type GLfloat = super::__gl_imports::raw::c_float;
pub type GLclampf = super::__gl_imports::raw::c_float;
pub type GLdouble = super::__gl_imports::raw::c_double;
pub type GLclampd = super::__gl_imports::raw::c_double;
pub type GLeglImageOES = *const super::__gl_imports::raw::c_void;
pub type GLchar = super::__gl_imports::raw::c_char;
pub type GLcharARB = super::__gl_imports::raw::c_char;

#[cfg(target_os = "macos")]
pub type GLhandleARB = *const super::__gl_imports::raw::c_void;
#[cfg(not(target_os = "macos"))]
pub type GLhandleARB = super::__gl_imports::raw::c_uint;

pub type GLhalfARB = super::__gl_imports::raw::c_ushort;
pub type GLhalf = super::__gl_imports::raw::c_ushort;

// Must be 32 bits
pub type GLfixed = GLint;

pub type GLintptr = isize;
pub type GLsizeiptr = isize;
pub type GLint64 = i64;
pub type GLuint64 = u64;
pub type GLintptrARB = isize;
pub type GLsizeiptrARB = isize;
pub type GLint64EXT = i64;
pub type GLuint64EXT = u64;

pub enum __GLsync {}
pub type GLsync = *const __GLsync;

// compatible with OpenCL cl_context
pub enum _cl_context {}
pub enum _cl_event {}

pub type GLDEBUGPROC = Option<extern "system" fn(source: GLenum,
                                                 gltype: GLenum,
                                                 id: GLuint,
                                                 severity: GLenum,
                                                 length: GLsizei,
                                                 message: *const GLchar,
                                                 userParam: *mut super::__gl_imports::raw::c_void)>;
pub type GLDEBUGPROCARB = Option<extern "system" fn(source: GLenum,
                                                    gltype: GLenum,
                                                    id: GLuint,
                                                    severity: GLenum,
                                                    length: GLsizei,
                                                    message: *const GLchar,
                                                    userParam: *mut super::__gl_imports::raw::c_void)>;
pub type GLDEBUGPROCKHR = Option<extern "system" fn(source: GLenum,
                                                    gltype: GLenum,
                                                    id: GLuint,
                                                    severity: GLenum,
                                                    length: GLsizei,
                                                    message: *const GLchar,
                                                    userParam: *mut super::__gl_imports::raw::c_void)>;

// GLES 1 types
// "pub type GLclampx = i32;",

// GLES 1/2 types (tagged for GLES 1)
// "pub type GLbyte = i8;",
// "pub type GLubyte = u8;",
// "pub type GLfloat = GLfloat;",
// "pub type GLclampf = GLfloat;",
// "pub type GLfixed = i32;",
// "pub type GLint64 = i64;",
// "pub type GLuint64 = u64;",
// "pub type GLintptr = intptr_t;",
// "pub type GLsizeiptr = ssize_t;",

// GLES 1/2 types (tagged for GLES 2 - attribute syntax is limited)
// "pub type GLbyte = i8;",
// "pub type GLubyte = u8;",
// "pub type GLfloat = GLfloat;",
// "pub type GLclampf = GLfloat;",
// "pub type GLfixed = i32;",
// "pub type GLint64 = i64;",
// "pub type GLuint64 = u64;",
// "pub type GLint64EXT = i64;",
// "pub type GLuint64EXT = u64;",
// "pub type GLintptr = intptr_t;",
// "pub type GLsizeiptr = ssize_t;",

// GLES 2 types (none currently)

// Vendor extension types
pub type GLDEBUGPROCAMD = Option<extern "system" fn(id: GLuint,
                                                    category: GLenum,
                                                    severity: GLenum,
                                                    length: GLsizei,
                                                    message: *const GLchar,
                                                    userParam: *mut super::__gl_imports::raw::c_void)>;
pub type GLhalfNV = super::__gl_imports::raw::c_ushort;
pub type GLvdpauSurfaceNV = GLintptr;


        }
    
#[allow(dead_code, non_upper_case_globals)] pub const ACTIVE_ATTRIBUTES: types::GLenum = 0x8B89;
#[allow(dead_code, non_upper_case_globals)] pub const ACTIVE_ATTRIBUTE_MAX_LENGTH: types::GLenum = 0x8B8A;
#[allow(dead_code, non_upper_case_globals)] pub const ACTIVE_TEXTURE: types::GLenum = 0x84E0;
#[allow(dead_code, non_upper_case_globals)] pub const ACTIVE_UNIFORMS: types::GLenum = 0x8B86;
#[allow(dead_code, non_upper_case_globals)] pub const ACTIVE_UNIFORM_BLOCKS: types::GLenum = 0x8A36;
#[allow(dead_code, non_upper_case_globals)] pub const ACTIVE_UNIFORM_BLOCK_MAX_NAME_LENGTH: types::GLenum = 0x8A35;
#[allow(dead_code, non_upper_case_globals)] pub const ACTIVE_UNIFORM_MAX_LENGTH: types::GLenum = 0x8B87;
#[allow(dead_code, non_upper_case_globals)] pub const ALIASED_LINE_WIDTH_RANGE: types::GLenum = 0x846E;
#[allow(dead_code, non_upper_case_globals)] pub const ALPHA: types::GLenum = 0x1906;
#[allow(dead_code, non_upper_case_globals)] pub const ALREADY_SIGNALED: types::GLenum = 0x911A;
#[allow(dead_code, non_upper_case_globals)] pub const ALWAYS: types::GLenum = 0x0207;
#[allow(dead_code, non_upper_case_globals)] pub const AND: types::GLenum = 0x1501;
#[allow(dead_code, non_upper_case_globals)] pub const AND_INVERTED: types::GLenum = 0x1504;
#[allow(dead_code, non_upper_case_globals)] pub const AND_REVERSE: types::GLenum = 0x1502;
#[allow(dead_code, non_upper_case_globals)] pub const ANY_SAMPLES_PASSED: types::GLenum = 0x8C2F;
#[allow(dead_code, non_upper_case_globals)] pub const ARRAY_BUFFER: types::GLenum = 0x8892;
#[allow(dead_code, non_upper_case_globals)] pub const ARRAY_BUFFER_BINDING: types::GLenum = 0x8894;
#[allow(dead_code, non_upper_case_globals)] pub const ATTACHED_SHADERS: types::GLenum = 0x8B85;
#[allow(dead_code, non_upper_case_globals)] pub const BACK: types::GLenum = 0x0405;
#[allow(dead_code, non_upper_case_globals)] pub const BACK_LEFT: types::GLenum = 0x0402;
#[allow(dead_code, non_upper_case_globals)] pub const BACK_RIGHT: types::GLenum = 0x0403;
#[allow(dead_code, non_upper_case_globals)] pub const BGR: types::GLenum = 0x80E0;
#[allow(dead_code, non_upper_case_globals)] pub const BGRA: types::GLenum = 0x80E1;
#[allow(dead_code, non_upper_case_globals)] pub const BGRA_INTEGER: types::GLenum = 0x8D9B;
#[allow(dead_code, non_upper_case_globals)] pub const BGR_INTEGER: types::GLenum = 0x8D9A;
#[allow(dead_code, non_upper_case_globals)] pub const BLEND: types::GLenum = 0x0BE2;
#[allow(dead_code, non_upper_case_globals)] pub const BLEND_COLOR: types::GLenum = 0x8005;
#[allow(dead_code, non_upper_case_globals)] pub const BLEND_DST: types::GLenum = 0x0BE0;
#[allow(dead_code, non_upper_case_globals)] pub const BLEND_DST_ALPHA: types::GLenum = 0x80CA;
#[allow(dead_code, non_upper_case_globals)] pub const BLEND_DST_RGB: types::GLenum = 0x80C8;
#[allow(dead_code, non_upper_case_globals)] pub const BLEND_EQUATION: types::GLenum = 0x8009;
#[allow(dead_code, non_upper_case_globals)] pub const BLEND_EQUATION_ALPHA: types::GLenum = 0x883D;
#[allow(dead_code, non_upper_case_globals)] pub const BLEND_EQUATION_RGB: types::GLenum = 0x8009;
#[allow(dead_code, non_upper_case_globals)] pub const BLEND_SRC: types::GLenum = 0x0BE1;
#[allow(dead_code, non_upper_case_globals)] pub const BLEND_SRC_ALPHA: types::GLenum = 0x80CB;
#[allow(dead_code, non_upper_case_globals)] pub const BLEND_SRC_RGB: types::GLenum = 0x80C9;
#[allow(dead_code, non_upper_case_globals)] pub const BLUE: types::GLenum = 0x1905;
#[allow(dead_code, non_upper_case_globals)] pub const BLUE_INTEGER: types::GLenum = 0x8D96;
#[allow(dead_code, non_upper_case_globals)] pub const BOOL: types::GLenum = 0x8B56;
#[allow(dead_code, non_upper_case_globals)] pub const BOOL_VEC2: types::GLenum = 0x8B57;
#[allow(dead_code, non_upper_case_globals)] pub const BOOL_VEC3: types::GLenum = 0x8B58;
#[allow(dead_code, non_upper_case_globals)] pub const BOOL_VEC4: types::GLenum = 0x8B59;
#[allow(dead_code, non_upper_case_globals)] pub const BUFFER_ACCESS: types::GLenum = 0x88BB;
#[allow(dead_code, non_upper_case_globals)] pub const BUFFER_ACCESS_FLAGS: types::GLenum = 0x911F;
#[allow(dead_code, non_upper_case_globals)] pub const BUFFER_MAPPED: types::GLenum = 0x88BC;
#[allow(dead_code, non_upper_case_globals)] pub const BUFFER_MAP_LENGTH: types::GLenum = 0x9120;
#[allow(dead_code, non_upper_case_globals)] pub const BUFFER_MAP_OFFSET: types::GLenum = 0x9121;
#[allow(dead_code, non_upper_case_globals)] pub const BUFFER_MAP_POINTER: types::GLenum = 0x88BD;
#[allow(dead_code, non_upper_case_globals)] pub const BUFFER_SIZE: types::GLenum = 0x8764;
#[allow(dead_code, non_upper_case_globals)] pub const BUFFER_USAGE: types::GLenum = 0x8765;
#[allow(dead_code, non_upper_case_globals)] pub const BYTE: types::GLenum = 0x1400;
#[allow(dead_code, non_upper_case_globals)] pub const CCW: types::GLenum = 0x0901;
#[allow(dead_code, non_upper_case_globals)] pub const CLAMP_READ_COLOR: types::GLenum = 0x891C;
#[allow(dead_code, non_upper_case_globals)] pub const CLAMP_TO_BORDER: types::GLenum = 0x812D;
#[allow(dead_code, non_upper_case_globals)] pub const CLAMP_TO_EDGE: types::GLenum = 0x812F;
#[allow(dead_code, non_upper_case_globals)] pub const CLEAR: types::GLenum = 0x1500;
#[allow(dead_code, non_upper_case_globals)] pub const CLIP_DISTANCE0: types::GLenum = 0x3000;
#[allow(dead_code, non_upper_case_globals)] pub const CLIP_DISTANCE1: types::GLenum = 0x3001;
#[allow(dead_code, non_upper_case_globals)] pub const CLIP_DISTANCE2: types::GLenum = 0x3002;
#[allow(dead_code, non_upper_case_globals)] pub const CLIP_DISTANCE3: types::GLenum = 0x3003;
#[allow(dead_code, non_upper_case_globals)] pub const CLIP_DISTANCE4: types::GLenum = 0x3004;
#[allow(dead_code, non_upper_case_globals)] pub const CLIP_DISTANCE5: types::GLenum = 0x3005;
#[allow(dead_code, non_upper_case_globals)] pub const CLIP_DISTANCE6: types::GLenum = 0x3006;
#[allow(dead_code, non_upper_case_globals)] pub const CLIP_DISTANCE7: types::GLenum = 0x3007;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR: types::GLenum = 0x1800;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT0: types::GLenum = 0x8CE0;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT1: types::GLenum = 0x8CE1;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT10: types::GLenum = 0x8CEA;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT11: types::GLenum = 0x8CEB;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT12: types::GLenum = 0x8CEC;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT13: types::GLenum = 0x8CED;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT14: types::GLenum = 0x8CEE;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT15: types::GLenum = 0x8CEF;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT16: types::GLenum = 0x8CF0;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT17: types::GLenum = 0x8CF1;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT18: types::GLenum = 0x8CF2;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT19: types::GLenum = 0x8CF3;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT2: types::GLenum = 0x8CE2;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT20: types::GLenum = 0x8CF4;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT21: types::GLenum = 0x8CF5;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT22: types::GLenum = 0x8CF6;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT23: types::GLenum = 0x8CF7;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT24: types::GLenum = 0x8CF8;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT25: types::GLenum = 0x8CF9;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT26: types::GLenum = 0x8CFA;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT27: types::GLenum = 0x8CFB;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT28: types::GLenum = 0x8CFC;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT29: types::GLenum = 0x8CFD;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT3: types::GLenum = 0x8CE3;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT30: types::GLenum = 0x8CFE;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT31: types::GLenum = 0x8CFF;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT4: types::GLenum = 0x8CE4;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT5: types::GLenum = 0x8CE5;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT6: types::GLenum = 0x8CE6;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT7: types::GLenum = 0x8CE7;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT8: types::GLenum = 0x8CE8;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_ATTACHMENT9: types::GLenum = 0x8CE9;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_BUFFER_BIT: types::GLenum = 0x00004000;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_CLEAR_VALUE: types::GLenum = 0x0C22;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_LOGIC_OP: types::GLenum = 0x0BF2;
#[allow(dead_code, non_upper_case_globals)] pub const COLOR_WRITEMASK: types::GLenum = 0x0C23;
#[allow(dead_code, non_upper_case_globals)] pub const COMPARE_REF_TO_TEXTURE: types::GLenum = 0x884E;
#[allow(dead_code, non_upper_case_globals)] pub const COMPILE_STATUS: types::GLenum = 0x8B81;
#[allow(dead_code, non_upper_case_globals)] pub const COMPRESSED_RED: types::GLenum = 0x8225;
#[allow(dead_code, non_upper_case_globals)] pub const COMPRESSED_RED_RGTC1: types::GLenum = 0x8DBB;
#[allow(dead_code, non_upper_case_globals)] pub const COMPRESSED_RG: types::GLenum = 0x8226;
#[allow(dead_code, non_upper_case_globals)] pub const COMPRESSED_RGB: types::GLenum = 0x84ED;
#[allow(dead_code, non_upper_case_globals)] pub const COMPRESSED_RGBA: types::GLenum = 0x84EE;
#[allow(dead_code, non_upper_case_globals)] pub const COMPRESSED_RG_RGTC2: types::GLenum = 0x8DBD;
#[allow(dead_code, non_upper_case_globals)] pub const COMPRESSED_SIGNED_RED_RGTC1: types::GLenum = 0x8DBC;
#[allow(dead_code, non_upper_case_globals)] pub const COMPRESSED_SIGNED_RG_RGTC2: types::GLenum = 0x8DBE;
#[allow(dead_code, non_upper_case_globals)] pub const COMPRESSED_SRGB: types::GLenum = 0x8C48;
#[allow(dead_code, non_upper_case_globals)] pub const COMPRESSED_SRGB_ALPHA: types::GLenum = 0x8C49;
#[allow(dead_code, non_upper_case_globals)] pub const COMPRESSED_TEXTURE_FORMATS: types::GLenum = 0x86A3;
#[allow(dead_code, non_upper_case_globals)] pub const CONDITION_SATISFIED: types::GLenum = 0x911C;
#[allow(dead_code, non_upper_case_globals)] pub const CONSTANT_ALPHA: types::GLenum = 0x8003;
#[allow(dead_code, non_upper_case_globals)] pub const CONSTANT_COLOR: types::GLenum = 0x8001;
#[allow(dead_code, non_upper_case_globals)] pub const CONTEXT_COMPATIBILITY_PROFILE_BIT: types::GLenum = 0x00000002;
#[allow(dead_code, non_upper_case_globals)] pub const CONTEXT_CORE_PROFILE_BIT: types::GLenum = 0x00000001;
#[allow(dead_code, non_upper_case_globals)] pub const CONTEXT_FLAGS: types::GLenum = 0x821E;
#[allow(dead_code, non_upper_case_globals)] pub const CONTEXT_FLAG_FORWARD_COMPATIBLE_BIT: types::GLenum = 0x00000001;
#[allow(dead_code, non_upper_case_globals)] pub const CONTEXT_PROFILE_MASK: types::GLenum = 0x9126;
#[allow(dead_code, non_upper_case_globals)] pub const COPY: types::GLenum = 0x1503;
#[allow(dead_code, non_upper_case_globals)] pub const COPY_INVERTED: types::GLenum = 0x150C;
#[allow(dead_code, non_upper_case_globals)] pub const COPY_READ_BUFFER: types::GLenum = 0x8F36;
#[allow(dead_code, non_upper_case_globals)] pub const COPY_WRITE_BUFFER: types::GLenum = 0x8F37;
#[allow(dead_code, non_upper_case_globals)] pub const CULL_FACE: types::GLenum = 0x0B44;
#[allow(dead_code, non_upper_case_globals)] pub const CULL_FACE_MODE: types::GLenum = 0x0B45;
#[allow(dead_code, non_upper_case_globals)] pub const CURRENT_PROGRAM: types::GLenum = 0x8B8D;
#[allow(dead_code, non_upper_case_globals)] pub const CURRENT_QUERY: types::GLenum = 0x8865;
#[allow(dead_code, non_upper_case_globals)] pub const CURRENT_VERTEX_ATTRIB: types::GLenum = 0x8626;
#[allow(dead_code, non_upper_case_globals)] pub const CW: types::GLenum = 0x0900;
#[allow(dead_code, non_upper_case_globals)] pub const DECR: types::GLenum = 0x1E03;
#[allow(dead_code, non_upper_case_globals)] pub const DECR_WRAP: types::GLenum = 0x8508;
#[allow(dead_code, non_upper_case_globals)] pub const DELETE_STATUS: types::GLenum = 0x8B80;
#[allow(dead_code, non_upper_case_globals)] pub const DEPTH: types::GLenum = 0x1801;
#[allow(dead_code, non_upper_case_globals)] pub const DEPTH24_STENCIL8: types::GLenum = 0x88F0;
#[allow(dead_code, non_upper_case_globals)] pub const DEPTH32F_STENCIL8: types::GLenum = 0x8CAD;
#[allow(dead_code, non_upper_case_globals)] pub const DEPTH_ATTACHMENT: types::GLenum = 0x8D00;
#[allow(dead_code, non_upper_case_globals)] pub const DEPTH_BUFFER_BIT: types::GLenum = 0x00000100;
#[allow(dead_code, non_upper_case_globals)] pub const DEPTH_CLAMP: types::GLenum = 0x864F;
#[allow(dead_code, non_upper_case_globals)] pub const DEPTH_CLEAR_VALUE: types::GLenum = 0x0B73;
#[allow(dead_code, non_upper_case_globals)] pub const DEPTH_COMPONENT: types::GLenum = 0x1902;
#[allow(dead_code, non_upper_case_globals)] pub const DEPTH_COMPONENT16: types::GLenum = 0x81A5;
#[allow(dead_code, non_upper_case_globals)] pub const DEPTH_COMPONENT24: types::GLenum = 0x81A6;
#[allow(dead_code, non_upper_case_globals)] pub const DEPTH_COMPONENT32: types::GLenum = 0x81A7;
#[allow(dead_code, non_upper_case_globals)] pub const DEPTH_COMPONENT32F: types::GLenum = 0x8CAC;
#[allow(dead_code, non_upper_case_globals)] pub const DEPTH_FUNC: types::GLenum = 0x0B74;
#[allow(dead_code, non_upper_case_globals)] pub const DEPTH_RANGE: types::GLenum = 0x0B70;
#[allow(dead_code, non_upper_case_globals)] pub const DEPTH_STENCIL: types::GLenum = 0x84F9;
#[allow(dead_code, non_upper_case_globals)] pub const DEPTH_STENCIL_ATTACHMENT: types::GLenum = 0x821A;
#[allow(dead_code, non_upper_case_globals)] pub const DEPTH_TEST: types::GLenum = 0x0B71;
#[allow(dead_code, non_upper_case_globals)] pub const DEPTH_WRITEMASK: types::GLenum = 0x0B72;
#[allow(dead_code, non_upper_case_globals)] pub const DITHER: types::GLenum = 0x0BD0;
#[allow(dead_code, non_upper_case_globals)] pub const DONT_CARE: types::GLenum = 0x1100;
#[allow(dead_code, non_upper_case_globals)] pub const DOUBLE: types::GLenum = 0x140A;
#[allow(dead_code, non_upper_case_globals)] pub const DOUBLEBUFFER: types::GLenum = 0x0C32;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_BUFFER: types::GLenum = 0x0C01;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_BUFFER0: types::GLenum = 0x8825;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_BUFFER1: types::GLenum = 0x8826;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_BUFFER10: types::GLenum = 0x882F;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_BUFFER11: types::GLenum = 0x8830;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_BUFFER12: types::GLenum = 0x8831;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_BUFFER13: types::GLenum = 0x8832;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_BUFFER14: types::GLenum = 0x8833;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_BUFFER15: types::GLenum = 0x8834;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_BUFFER2: types::GLenum = 0x8827;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_BUFFER3: types::GLenum = 0x8828;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_BUFFER4: types::GLenum = 0x8829;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_BUFFER5: types::GLenum = 0x882A;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_BUFFER6: types::GLenum = 0x882B;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_BUFFER7: types::GLenum = 0x882C;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_BUFFER8: types::GLenum = 0x882D;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_BUFFER9: types::GLenum = 0x882E;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_FRAMEBUFFER: types::GLenum = 0x8CA9;
#[allow(dead_code, non_upper_case_globals)] pub const DRAW_FRAMEBUFFER_BINDING: types::GLenum = 0x8CA6;
#[allow(dead_code, non_upper_case_globals)] pub const DST_ALPHA: types::GLenum = 0x0304;
#[allow(dead_code, non_upper_case_globals)] pub const DST_COLOR: types::GLenum = 0x0306;
#[allow(dead_code, non_upper_case_globals)] pub const DYNAMIC_COPY: types::GLenum = 0x88EA;
#[allow(dead_code, non_upper_case_globals)] pub const DYNAMIC_DRAW: types::GLenum = 0x88E8;
#[allow(dead_code, non_upper_case_globals)] pub const DYNAMIC_READ: types::GLenum = 0x88E9;
#[allow(dead_code, non_upper_case_globals)] pub const ELEMENT_ARRAY_BUFFER: types::GLenum = 0x8893;
#[allow(dead_code, non_upper_case_globals)] pub const ELEMENT_ARRAY_BUFFER_BINDING: types::GLenum = 0x8895;
#[allow(dead_code, non_upper_case_globals)] pub const EQUAL: types::GLenum = 0x0202;
#[allow(dead_code, non_upper_case_globals)] pub const EQUIV: types::GLenum = 0x1509;
#[allow(dead_code, non_upper_case_globals)] pub const EXTENSIONS: types::GLenum = 0x1F03;
#[allow(dead_code, non_upper_case_globals)] pub const FALSE: types::GLboolean = 0;
#[allow(dead_code, non_upper_case_globals)] pub const FASTEST: types::GLenum = 0x1101;
#[allow(dead_code, non_upper_case_globals)] pub const FILL: types::GLenum = 0x1B02;
#[allow(dead_code, non_upper_case_globals)] pub const FIRST_VERTEX_CONVENTION: types::GLenum = 0x8E4D;
#[allow(dead_code, non_upper_case_globals)] pub const FIXED_ONLY: types::GLenum = 0x891D;
#[allow(dead_code, non_upper_case_globals)] pub const FLOAT: types::GLenum = 0x1406;
#[allow(dead_code, non_upper_case_globals)] pub const FLOAT_32_UNSIGNED_INT_24_8_REV: types::GLenum = 0x8DAD;
#[allow(dead_code, non_upper_case_globals)] pub const FLOAT_MAT2: types::GLenum = 0x8B5A;
#[allow(dead_code, non_upper_case_globals)] pub const FLOAT_MAT2x3: types::GLenum = 0x8B65;
#[allow(dead_code, non_upper_case_globals)] pub const FLOAT_MAT2x4: types::GLenum = 0x8B66;
#[allow(dead_code, non_upper_case_globals)] pub const FLOAT_MAT3: types::GLenum = 0x8B5B;
#[allow(dead_code, non_upper_case_globals)] pub const FLOAT_MAT3x2: types::GLenum = 0x8B67;
#[allow(dead_code, non_upper_case_globals)] pub const FLOAT_MAT3x4: types::GLenum = 0x8B68;
#[allow(dead_code, non_upper_case_globals)] pub const FLOAT_MAT4: types::GLenum = 0x8B5C;
#[allow(dead_code, non_upper_case_globals)] pub const FLOAT_MAT4x2: types::GLenum = 0x8B69;
#[allow(dead_code, non_upper_case_globals)] pub const FLOAT_MAT4x3: types::GLenum = 0x8B6A;
#[allow(dead_code, non_upper_case_globals)] pub const FLOAT_VEC2: types::GLenum = 0x8B50;
#[allow(dead_code, non_upper_case_globals)] pub const FLOAT_VEC3: types::GLenum = 0x8B51;
#[allow(dead_code, non_upper_case_globals)] pub const FLOAT_VEC4: types::GLenum = 0x8B52;
#[allow(dead_code, non_upper_case_globals)] pub const FRAGMENT_SHADER: types::GLenum = 0x8B30;
#[allow(dead_code, non_upper_case_globals)] pub const FRAGMENT_SHADER_DERIVATIVE_HINT: types::GLenum = 0x8B8B;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER: types::GLenum = 0x8D40;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_ATTACHMENT_ALPHA_SIZE: types::GLenum = 0x8215;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_ATTACHMENT_BLUE_SIZE: types::GLenum = 0x8214;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_ATTACHMENT_COLOR_ENCODING: types::GLenum = 0x8210;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_ATTACHMENT_COMPONENT_TYPE: types::GLenum = 0x8211;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_ATTACHMENT_DEPTH_SIZE: types::GLenum = 0x8216;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_ATTACHMENT_GREEN_SIZE: types::GLenum = 0x8213;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_ATTACHMENT_LAYERED: types::GLenum = 0x8DA7;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_ATTACHMENT_OBJECT_NAME: types::GLenum = 0x8CD1;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_ATTACHMENT_OBJECT_TYPE: types::GLenum = 0x8CD0;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_ATTACHMENT_RED_SIZE: types::GLenum = 0x8212;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_ATTACHMENT_STENCIL_SIZE: types::GLenum = 0x8217;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_ATTACHMENT_TEXTURE_CUBE_MAP_FACE: types::GLenum = 0x8CD3;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_ATTACHMENT_TEXTURE_LAYER: types::GLenum = 0x8CD4;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_ATTACHMENT_TEXTURE_LEVEL: types::GLenum = 0x8CD2;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_BINDING: types::GLenum = 0x8CA6;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_COMPLETE: types::GLenum = 0x8CD5;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_DEFAULT: types::GLenum = 0x8218;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_INCOMPLETE_ATTACHMENT: types::GLenum = 0x8CD6;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_INCOMPLETE_DRAW_BUFFER: types::GLenum = 0x8CDB;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_INCOMPLETE_LAYER_TARGETS: types::GLenum = 0x8DA8;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT: types::GLenum = 0x8CD7;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_INCOMPLETE_MULTISAMPLE: types::GLenum = 0x8D56;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_INCOMPLETE_READ_BUFFER: types::GLenum = 0x8CDC;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_SRGB: types::GLenum = 0x8DB9;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_UNDEFINED: types::GLenum = 0x8219;
#[allow(dead_code, non_upper_case_globals)] pub const FRAMEBUFFER_UNSUPPORTED: types::GLenum = 0x8CDD;
#[allow(dead_code, non_upper_case_globals)] pub const FRONT: types::GLenum = 0x0404;
#[allow(dead_code, non_upper_case_globals)] pub const FRONT_AND_BACK: types::GLenum = 0x0408;
#[allow(dead_code, non_upper_case_globals)] pub const FRONT_FACE: types::GLenum = 0x0B46;
#[allow(dead_code, non_upper_case_globals)] pub const FRONT_LEFT: types::GLenum = 0x0400;
#[allow(dead_code, non_upper_case_globals)] pub const FRONT_RIGHT: types::GLenum = 0x0401;
#[allow(dead_code, non_upper_case_globals)] pub const FUNC_ADD: types::GLenum = 0x8006;
#[allow(dead_code, non_upper_case_globals)] pub const FUNC_REVERSE_SUBTRACT: types::GLenum = 0x800B;
#[allow(dead_code, non_upper_case_globals)] pub const FUNC_SUBTRACT: types::GLenum = 0x800A;
#[allow(dead_code, non_upper_case_globals)] pub const GEOMETRY_INPUT_TYPE: types::GLenum = 0x8917;
#[allow(dead_code, non_upper_case_globals)] pub const GEOMETRY_OUTPUT_TYPE: types::GLenum = 0x8918;
#[allow(dead_code, non_upper_case_globals)] pub const GEOMETRY_SHADER: types::GLenum = 0x8DD9;
#[allow(dead_code, non_upper_case_globals)] pub const GEOMETRY_VERTICES_OUT: types::GLenum = 0x8916;
#[allow(dead_code, non_upper_case_globals)] pub const GEQUAL: types::GLenum = 0x0206;
#[allow(dead_code, non_upper_case_globals)] pub const GREATER: types::GLenum = 0x0204;
#[allow(dead_code, non_upper_case_globals)] pub const GREEN: types::GLenum = 0x1904;
#[allow(dead_code, non_upper_case_globals)] pub const GREEN_INTEGER: types::GLenum = 0x8D95;
#[allow(dead_code, non_upper_case_globals)] pub const HALF_FLOAT: types::GLenum = 0x140B;
#[allow(dead_code, non_upper_case_globals)] pub const INCR: types::GLenum = 0x1E02;
#[allow(dead_code, non_upper_case_globals)] pub const INCR_WRAP: types::GLenum = 0x8507;
#[allow(dead_code, non_upper_case_globals)] pub const INFO_LOG_LENGTH: types::GLenum = 0x8B84;
#[allow(dead_code, non_upper_case_globals)] pub const INT: types::GLenum = 0x1404;
#[allow(dead_code, non_upper_case_globals)] pub const INTERLEAVED_ATTRIBS: types::GLenum = 0x8C8C;
#[allow(dead_code, non_upper_case_globals)] pub const INT_2_10_10_10_REV: types::GLenum = 0x8D9F;
#[allow(dead_code, non_upper_case_globals)] pub const INT_SAMPLER_1D: types::GLenum = 0x8DC9;
#[allow(dead_code, non_upper_case_globals)] pub const INT_SAMPLER_1D_ARRAY: types::GLenum = 0x8DCE;
#[allow(dead_code, non_upper_case_globals)] pub const INT_SAMPLER_2D: types::GLenum = 0x8DCA;
#[allow(dead_code, non_upper_case_globals)] pub const INT_SAMPLER_2D_ARRAY: types::GLenum = 0x8DCF;
#[allow(dead_code, non_upper_case_globals)] pub const INT_SAMPLER_2D_MULTISAMPLE: types::GLenum = 0x9109;
#[allow(dead_code, non_upper_case_globals)] pub const INT_SAMPLER_2D_MULTISAMPLE_ARRAY: types::GLenum = 0x910C;
#[allow(dead_code, non_upper_case_globals)] pub const INT_SAMPLER_2D_RECT: types::GLenum = 0x8DCD;
#[allow(dead_code, non_upper_case_globals)] pub const INT_SAMPLER_3D: types::GLenum = 0x8DCB;
#[allow(dead_code, non_upper_case_globals)] pub const INT_SAMPLER_BUFFER: types::GLenum = 0x8DD0;
#[allow(dead_code, non_upper_case_globals)] pub const INT_SAMPLER_CUBE: types::GLenum = 0x8DCC;
#[allow(dead_code, non_upper_case_globals)] pub const INT_VEC2: types::GLenum = 0x8B53;
#[allow(dead_code, non_upper_case_globals)] pub const INT_VEC3: types::GLenum = 0x8B54;
#[allow(dead_code, non_upper_case_globals)] pub const INT_VEC4: types::GLenum = 0x8B55;
#[allow(dead_code, non_upper_case_globals)] pub const INVALID_ENUM: types::GLenum = 0x0500;
#[allow(dead_code, non_upper_case_globals)] pub const INVALID_FRAMEBUFFER_OPERATION: types::GLenum = 0x0506;
#[allow(dead_code, non_upper_case_globals)] pub const INVALID_INDEX: types::GLuint = 0xFFFFFFFF;
#[allow(dead_code, non_upper_case_globals)] pub const INVALID_OPERATION: types::GLenum = 0x0502;
#[allow(dead_code, non_upper_case_globals)] pub const INVALID_VALUE: types::GLenum = 0x0501;
#[allow(dead_code, non_upper_case_globals)] pub const INVERT: types::GLenum = 0x150A;
#[allow(dead_code, non_upper_case_globals)] pub const KEEP: types::GLenum = 0x1E00;
#[allow(dead_code, non_upper_case_globals)] pub const LAST_VERTEX_CONVENTION: types::GLenum = 0x8E4E;
#[allow(dead_code, non_upper_case_globals)] pub const LEFT: types::GLenum = 0x0406;
#[allow(dead_code, non_upper_case_globals)] pub const LEQUAL: types::GLenum = 0x0203;
#[allow(dead_code, non_upper_case_globals)] pub const LESS: types::GLenum = 0x0201;
#[allow(dead_code, non_upper_case_globals)] pub const LINE: types::GLenum = 0x1B01;
#[allow(dead_code, non_upper_case_globals)] pub const LINEAR: types::GLenum = 0x2601;
#[allow(dead_code, non_upper_case_globals)] pub const LINEAR_MIPMAP_LINEAR: types::GLenum = 0x2703;
#[allow(dead_code, non_upper_case_globals)] pub const LINEAR_MIPMAP_NEAREST: types::GLenum = 0x2701;
#[allow(dead_code, non_upper_case_globals)] pub const LINES: types::GLenum = 0x0001;
#[allow(dead_code, non_upper_case_globals)] pub const LINES_ADJACENCY: types::GLenum = 0x000A;
#[allow(dead_code, non_upper_case_globals)] pub const LINE_LOOP: types::GLenum = 0x0002;
#[allow(dead_code, non_upper_case_globals)] pub const LINE_SMOOTH: types::GLenum = 0x0B20;
#[allow(dead_code, non_upper_case_globals)] pub const LINE_SMOOTH_HINT: types::GLenum = 0x0C52;
#[allow(dead_code, non_upper_case_globals)] pub const LINE_STRIP: types::GLenum = 0x0003;
#[allow(dead_code, non_upper_case_globals)] pub const LINE_STRIP_ADJACENCY: types::GLenum = 0x000B;
#[allow(dead_code, non_upper_case_globals)] pub const LINE_WIDTH: types::GLenum = 0x0B21;
#[allow(dead_code, non_upper_case_globals)] pub const LINE_WIDTH_GRANULARITY: types::GLenum = 0x0B23;
#[allow(dead_code, non_upper_case_globals)] pub const LINE_WIDTH_RANGE: types::GLenum = 0x0B22;
#[allow(dead_code, non_upper_case_globals)] pub const LINK_STATUS: types::GLenum = 0x8B82;
#[allow(dead_code, non_upper_case_globals)] pub const LOGIC_OP_MODE: types::GLenum = 0x0BF0;
#[allow(dead_code, non_upper_case_globals)] pub const LOWER_LEFT: types::GLenum = 0x8CA1;
#[allow(dead_code, non_upper_case_globals)] pub const MAJOR_VERSION: types::GLenum = 0x821B;
#[allow(dead_code, non_upper_case_globals)] pub const MAP_FLUSH_EXPLICIT_BIT: types::GLenum = 0x0010;
#[allow(dead_code, non_upper_case_globals)] pub const MAP_INVALIDATE_BUFFER_BIT: types::GLenum = 0x0008;
#[allow(dead_code, non_upper_case_globals)] pub const MAP_INVALIDATE_RANGE_BIT: types::GLenum = 0x0004;
#[allow(dead_code, non_upper_case_globals)] pub const MAP_READ_BIT: types::GLenum = 0x0001;
#[allow(dead_code, non_upper_case_globals)] pub const MAP_UNSYNCHRONIZED_BIT: types::GLenum = 0x0020;
#[allow(dead_code, non_upper_case_globals)] pub const MAP_WRITE_BIT: types::GLenum = 0x0002;
#[allow(dead_code, non_upper_case_globals)] pub const MAX: types::GLenum = 0x8008;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_3D_TEXTURE_SIZE: types::GLenum = 0x8073;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_ARRAY_TEXTURE_LAYERS: types::GLenum = 0x88FF;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_CLIP_DISTANCES: types::GLenum = 0x0D32;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_COLOR_ATTACHMENTS: types::GLenum = 0x8CDF;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_COLOR_TEXTURE_SAMPLES: types::GLenum = 0x910E;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_COMBINED_FRAGMENT_UNIFORM_COMPONENTS: types::GLenum = 0x8A33;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_COMBINED_GEOMETRY_UNIFORM_COMPONENTS: types::GLenum = 0x8A32;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_COMBINED_TEXTURE_IMAGE_UNITS: types::GLenum = 0x8B4D;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_COMBINED_UNIFORM_BLOCKS: types::GLenum = 0x8A2E;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_COMBINED_VERTEX_UNIFORM_COMPONENTS: types::GLenum = 0x8A31;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_CUBE_MAP_TEXTURE_SIZE: types::GLenum = 0x851C;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_DEPTH_TEXTURE_SAMPLES: types::GLenum = 0x910F;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_DRAW_BUFFERS: types::GLenum = 0x8824;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_DUAL_SOURCE_DRAW_BUFFERS: types::GLenum = 0x88FC;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_ELEMENTS_INDICES: types::GLenum = 0x80E9;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_ELEMENTS_VERTICES: types::GLenum = 0x80E8;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_FRAGMENT_INPUT_COMPONENTS: types::GLenum = 0x9125;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_FRAGMENT_UNIFORM_BLOCKS: types::GLenum = 0x8A2D;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_FRAGMENT_UNIFORM_COMPONENTS: types::GLenum = 0x8B49;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_GEOMETRY_INPUT_COMPONENTS: types::GLenum = 0x9123;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_GEOMETRY_OUTPUT_COMPONENTS: types::GLenum = 0x9124;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_GEOMETRY_OUTPUT_VERTICES: types::GLenum = 0x8DE0;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_GEOMETRY_TEXTURE_IMAGE_UNITS: types::GLenum = 0x8C29;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_GEOMETRY_TOTAL_OUTPUT_COMPONENTS: types::GLenum = 0x8DE1;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_GEOMETRY_UNIFORM_BLOCKS: types::GLenum = 0x8A2C;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_GEOMETRY_UNIFORM_COMPONENTS: types::GLenum = 0x8DDF;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_INTEGER_SAMPLES: types::GLenum = 0x9110;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_PROGRAM_TEXEL_OFFSET: types::GLenum = 0x8905;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_RECTANGLE_TEXTURE_SIZE: types::GLenum = 0x84F8;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_RENDERBUFFER_SIZE: types::GLenum = 0x84E8;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_SAMPLES: types::GLenum = 0x8D57;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_SAMPLE_MASK_WORDS: types::GLenum = 0x8E59;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_SERVER_WAIT_TIMEOUT: types::GLenum = 0x9111;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_TEXTURE_BUFFER_SIZE: types::GLenum = 0x8C2B;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_TEXTURE_IMAGE_UNITS: types::GLenum = 0x8872;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_TEXTURE_LOD_BIAS: types::GLenum = 0x84FD;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_TEXTURE_SIZE: types::GLenum = 0x0D33;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_TRANSFORM_FEEDBACK_INTERLEAVED_COMPONENTS: types::GLenum = 0x8C8A;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_TRANSFORM_FEEDBACK_SEPARATE_ATTRIBS: types::GLenum = 0x8C8B;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_TRANSFORM_FEEDBACK_SEPARATE_COMPONENTS: types::GLenum = 0x8C80;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_UNIFORM_BLOCK_SIZE: types::GLenum = 0x8A30;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_UNIFORM_BUFFER_BINDINGS: types::GLenum = 0x8A2F;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_VARYING_COMPONENTS: types::GLenum = 0x8B4B;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_VARYING_FLOATS: types::GLenum = 0x8B4B;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_VERTEX_ATTRIBS: types::GLenum = 0x8869;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_VERTEX_OUTPUT_COMPONENTS: types::GLenum = 0x9122;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_VERTEX_TEXTURE_IMAGE_UNITS: types::GLenum = 0x8B4C;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_VERTEX_UNIFORM_BLOCKS: types::GLenum = 0x8A2B;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_VERTEX_UNIFORM_COMPONENTS: types::GLenum = 0x8B4A;
#[allow(dead_code, non_upper_case_globals)] pub const MAX_VIEWPORT_DIMS: types::GLenum = 0x0D3A;
#[allow(dead_code, non_upper_case_globals)] pub const MIN: types::GLenum = 0x8007;
#[allow(dead_code, non_upper_case_globals)] pub const MINOR_VERSION: types::GLenum = 0x821C;
#[allow(dead_code, non_upper_case_globals)] pub const MIN_PROGRAM_TEXEL_OFFSET: types::GLenum = 0x8904;
#[allow(dead_code, non_upper_case_globals)] pub const MIRRORED_REPEAT: types::GLenum = 0x8370;
#[allow(dead_code, non_upper_case_globals)] pub const MULTISAMPLE: types::GLenum = 0x809D;
#[allow(dead_code, non_upper_case_globals)] pub const NAND: types::GLenum = 0x150E;
#[allow(dead_code, non_upper_case_globals)] pub const NEAREST: types::GLenum = 0x2600;
#[allow(dead_code, non_upper_case_globals)] pub const NEAREST_MIPMAP_LINEAR: types::GLenum = 0x2702;
#[allow(dead_code, non_upper_case_globals)] pub const NEAREST_MIPMAP_NEAREST: types::GLenum = 0x2700;
#[allow(dead_code, non_upper_case_globals)] pub const NEVER: types::GLenum = 0x0200;
#[allow(dead_code, non_upper_case_globals)] pub const NICEST: types::GLenum = 0x1102;
#[allow(dead_code, non_upper_case_globals)] pub const NONE: types::GLenum = 0;
#[allow(dead_code, non_upper_case_globals)] pub const NOOP: types::GLenum = 0x1505;
#[allow(dead_code, non_upper_case_globals)] pub const NOR: types::GLenum = 0x1508;
#[allow(dead_code, non_upper_case_globals)] pub const NOTEQUAL: types::GLenum = 0x0205;
#[allow(dead_code, non_upper_case_globals)] pub const NO_ERROR: types::GLenum = 0;
#[allow(dead_code, non_upper_case_globals)] pub const NUM_COMPRESSED_TEXTURE_FORMATS: types::GLenum = 0x86A2;
#[allow(dead_code, non_upper_case_globals)] pub const NUM_EXTENSIONS: types::GLenum = 0x821D;
#[allow(dead_code, non_upper_case_globals)] pub const OBJECT_TYPE: types::GLenum = 0x9112;
#[allow(dead_code, non_upper_case_globals)] pub const ONE: types::GLenum = 1;
#[allow(dead_code, non_upper_case_globals)] pub const ONE_MINUS_CONSTANT_ALPHA: types::GLenum = 0x8004;
#[allow(dead_code, non_upper_case_globals)] pub const ONE_MINUS_CONSTANT_COLOR: types::GLenum = 0x8002;
#[allow(dead_code, non_upper_case_globals)] pub const ONE_MINUS_DST_ALPHA: types::GLenum = 0x0305;
#[allow(dead_code, non_upper_case_globals)] pub const ONE_MINUS_DST_COLOR: types::GLenum = 0x0307;
#[allow(dead_code, non_upper_case_globals)] pub const ONE_MINUS_SRC1_ALPHA: types::GLenum = 0x88FB;
#[allow(dead_code, non_upper_case_globals)] pub const ONE_MINUS_SRC1_COLOR: types::GLenum = 0x88FA;
#[allow(dead_code, non_upper_case_globals)] pub const ONE_MINUS_SRC_ALPHA: types::GLenum = 0x0303;
#[allow(dead_code, non_upper_case_globals)] pub const ONE_MINUS_SRC_COLOR: types::GLenum = 0x0301;
#[allow(dead_code, non_upper_case_globals)] pub const OR: types::GLenum = 0x1507;
#[allow(dead_code, non_upper_case_globals)] pub const OR_INVERTED: types::GLenum = 0x150D;
#[allow(dead_code, non_upper_case_globals)] pub const OR_REVERSE: types::GLenum = 0x150B;
#[allow(dead_code, non_upper_case_globals)] pub const OUT_OF_MEMORY: types::GLenum = 0x0505;
#[allow(dead_code, non_upper_case_globals)] pub const PACK_ALIGNMENT: types::GLenum = 0x0D05;
#[allow(dead_code, non_upper_case_globals)] pub const PACK_IMAGE_HEIGHT: types::GLenum = 0x806C;
#[allow(dead_code, non_upper_case_globals)] pub const PACK_LSB_FIRST: types::GLenum = 0x0D01;
#[allow(dead_code, non_upper_case_globals)] pub const PACK_ROW_LENGTH: types::GLenum = 0x0D02;
#[allow(dead_code, non_upper_case_globals)] pub const PACK_SKIP_IMAGES: types::GLenum = 0x806B;
#[allow(dead_code, non_upper_case_globals)] pub const PACK_SKIP_PIXELS: types::GLenum = 0x0D04;
#[allow(dead_code, non_upper_case_globals)] pub const PACK_SKIP_ROWS: types::GLenum = 0x0D03;
#[allow(dead_code, non_upper_case_globals)] pub const PACK_SWAP_BYTES: types::GLenum = 0x0D00;
#[allow(dead_code, non_upper_case_globals)] pub const PIXEL_PACK_BUFFER: types::GLenum = 0x88EB;
#[allow(dead_code, non_upper_case_globals)] pub const PIXEL_PACK_BUFFER_BINDING: types::GLenum = 0x88ED;
#[allow(dead_code, non_upper_case_globals)] pub const PIXEL_UNPACK_BUFFER: types::GLenum = 0x88EC;
#[allow(dead_code, non_upper_case_globals)] pub const PIXEL_UNPACK_BUFFER_BINDING: types::GLenum = 0x88EF;
#[allow(dead_code, non_upper_case_globals)] pub const POINT: types::GLenum = 0x1B00;
#[allow(dead_code, non_upper_case_globals)] pub const POINTS: types::GLenum = 0x0000;
#[allow(dead_code, non_upper_case_globals)] pub const POINT_FADE_THRESHOLD_SIZE: types::GLenum = 0x8128;
#[allow(dead_code, non_upper_case_globals)] pub const POINT_SIZE: types::GLenum = 0x0B11;
#[allow(dead_code, non_upper_case_globals)] pub const POINT_SIZE_GRANULARITY: types::GLenum = 0x0B13;
#[allow(dead_code, non_upper_case_globals)] pub const POINT_SIZE_RANGE: types::GLenum = 0x0B12;
#[allow(dead_code, non_upper_case_globals)] pub const POINT_SPRITE_COORD_ORIGIN: types::GLenum = 0x8CA0;
#[allow(dead_code, non_upper_case_globals)] pub const POLYGON_MODE: types::GLenum = 0x0B40;
#[allow(dead_code, non_upper_case_globals)] pub const POLYGON_OFFSET_FACTOR: types::GLenum = 0x8038;
#[allow(dead_code, non_upper_case_globals)] pub const POLYGON_OFFSET_FILL: types::GLenum = 0x8037;
#[allow(dead_code, non_upper_case_globals)] pub const POLYGON_OFFSET_LINE: types::GLenum = 0x2A02;
#[allow(dead_code, non_upper_case_globals)] pub const POLYGON_OFFSET_POINT: types::GLenum = 0x2A01;
#[allow(dead_code, non_upper_case_globals)] pub const POLYGON_OFFSET_UNITS: types::GLenum = 0x2A00;
#[allow(dead_code, non_upper_case_globals)] pub const POLYGON_SMOOTH: types::GLenum = 0x0B41;
#[allow(dead_code, non_upper_case_globals)] pub const POLYGON_SMOOTH_HINT: types::GLenum = 0x0C53;
#[allow(dead_code, non_upper_case_globals)] pub const PRIMITIVES_GENERATED: types::GLenum = 0x8C87;
#[allow(dead_code, non_upper_case_globals)] pub const PRIMITIVE_RESTART: types::GLenum = 0x8F9D;
#[allow(dead_code, non_upper_case_globals)] pub const PRIMITIVE_RESTART_INDEX: types::GLenum = 0x8F9E;
#[allow(dead_code, non_upper_case_globals)] pub const PROGRAM_POINT_SIZE: types::GLenum = 0x8642;
#[allow(dead_code, non_upper_case_globals)] pub const PROVOKING_VERTEX: types::GLenum = 0x8E4F;
#[allow(dead_code, non_upper_case_globals)] pub const PROXY_TEXTURE_1D: types::GLenum = 0x8063;
#[allow(dead_code, non_upper_case_globals)] pub const PROXY_TEXTURE_1D_ARRAY: types::GLenum = 0x8C19;
#[allow(dead_code, non_upper_case_globals)] pub const PROXY_TEXTURE_2D: types::GLenum = 0x8064;
#[allow(dead_code, non_upper_case_globals)] pub const PROXY_TEXTURE_2D_ARRAY: types::GLenum = 0x8C1B;
#[allow(dead_code, non_upper_case_globals)] pub const PROXY_TEXTURE_2D_MULTISAMPLE: types::GLenum = 0x9101;
#[allow(dead_code, non_upper_case_globals)] pub const PROXY_TEXTURE_2D_MULTISAMPLE_ARRAY: types::GLenum = 0x9103;
#[allow(dead_code, non_upper_case_globals)] pub const PROXY_TEXTURE_3D: types::GLenum = 0x8070;
#[allow(dead_code, non_upper_case_globals)] pub const PROXY_TEXTURE_CUBE_MAP: types::GLenum = 0x851B;
#[allow(dead_code, non_upper_case_globals)] pub const PROXY_TEXTURE_RECTANGLE: types::GLenum = 0x84F7;
#[allow(dead_code, non_upper_case_globals)] pub const QUADS_FOLLOW_PROVOKING_VERTEX_CONVENTION: types::GLenum = 0x8E4C;
#[allow(dead_code, non_upper_case_globals)] pub const QUERY_BY_REGION_NO_WAIT: types::GLenum = 0x8E16;
#[allow(dead_code, non_upper_case_globals)] pub const QUERY_BY_REGION_WAIT: types::GLenum = 0x8E15;
#[allow(dead_code, non_upper_case_globals)] pub const QUERY_COUNTER_BITS: types::GLenum = 0x8864;
#[allow(dead_code, non_upper_case_globals)] pub const QUERY_NO_WAIT: types::GLenum = 0x8E14;
#[allow(dead_code, non_upper_case_globals)] pub const QUERY_RESULT: types::GLenum = 0x8866;
#[allow(dead_code, non_upper_case_globals)] pub const QUERY_RESULT_AVAILABLE: types::GLenum = 0x8867;
#[allow(dead_code, non_upper_case_globals)] pub const QUERY_WAIT: types::GLenum = 0x8E13;
#[allow(dead_code, non_upper_case_globals)] pub const R11F_G11F_B10F: types::GLenum = 0x8C3A;
#[allow(dead_code, non_upper_case_globals)] pub const R16: types::GLenum = 0x822A;
#[allow(dead_code, non_upper_case_globals)] pub const R16F: types::GLenum = 0x822D;
#[allow(dead_code, non_upper_case_globals)] pub const R16I: types::GLenum = 0x8233;
#[allow(dead_code, non_upper_case_globals)] pub const R16UI: types::GLenum = 0x8234;
#[allow(dead_code, non_upper_case_globals)] pub const R16_SNORM: types::GLenum = 0x8F98;
#[allow(dead_code, non_upper_case_globals)] pub const R32F: types::GLenum = 0x822E;
#[allow(dead_code, non_upper_case_globals)] pub const R32I: types::GLenum = 0x8235;
#[allow(dead_code, non_upper_case_globals)] pub const R32UI: types::GLenum = 0x8236;
#[allow(dead_code, non_upper_case_globals)] pub const R3_G3_B2: types::GLenum = 0x2A10;
#[allow(dead_code, non_upper_case_globals)] pub const R8: types::GLenum = 0x8229;
#[allow(dead_code, non_upper_case_globals)] pub const R8I: types::GLenum = 0x8231;
#[allow(dead_code, non_upper_case_globals)] pub const R8UI: types::GLenum = 0x8232;
#[allow(dead_code, non_upper_case_globals)] pub const R8_SNORM: types::GLenum = 0x8F94;
#[allow(dead_code, non_upper_case_globals)] pub const RASTERIZER_DISCARD: types::GLenum = 0x8C89;
#[allow(dead_code, non_upper_case_globals)] pub const READ_BUFFER: types::GLenum = 0x0C02;
#[allow(dead_code, non_upper_case_globals)] pub const READ_FRAMEBUFFER: types::GLenum = 0x8CA8;
#[allow(dead_code, non_upper_case_globals)] pub const READ_FRAMEBUFFER_BINDING: types::GLenum = 0x8CAA;
#[allow(dead_code, non_upper_case_globals)] pub const READ_ONLY: types::GLenum = 0x88B8;
#[allow(dead_code, non_upper_case_globals)] pub const READ_WRITE: types::GLenum = 0x88BA;
#[allow(dead_code, non_upper_case_globals)] pub const RED: types::GLenum = 0x1903;
#[allow(dead_code, non_upper_case_globals)] pub const RED_INTEGER: types::GLenum = 0x8D94;
#[allow(dead_code, non_upper_case_globals)] pub const RENDERBUFFER: types::GLenum = 0x8D41;
#[allow(dead_code, non_upper_case_globals)] pub const RENDERBUFFER_ALPHA_SIZE: types::GLenum = 0x8D53;
#[allow(dead_code, non_upper_case_globals)] pub const RENDERBUFFER_BINDING: types::GLenum = 0x8CA7;
#[allow(dead_code, non_upper_case_globals)] pub const RENDERBUFFER_BLUE_SIZE: types::GLenum = 0x8D52;
#[allow(dead_code, non_upper_case_globals)] pub const RENDERBUFFER_DEPTH_SIZE: types::GLenum = 0x8D54;
#[allow(dead_code, non_upper_case_globals)] pub const RENDERBUFFER_GREEN_SIZE: types::GLenum = 0x8D51;
#[allow(dead_code, non_upper_case_globals)] pub const RENDERBUFFER_HEIGHT: types::GLenum = 0x8D43;
#[allow(dead_code, non_upper_case_globals)] pub const RENDERBUFFER_INTERNAL_FORMAT: types::GLenum = 0x8D44;
#[allow(dead_code, non_upper_case_globals)] pub const RENDERBUFFER_RED_SIZE: types::GLenum = 0x8D50;
#[allow(dead_code, non_upper_case_globals)] pub const RENDERBUFFER_SAMPLES: types::GLenum = 0x8CAB;
#[allow(dead_code, non_upper_case_globals)] pub const RENDERBUFFER_STENCIL_SIZE: types::GLenum = 0x8D55;
#[allow(dead_code, non_upper_case_globals)] pub const RENDERBUFFER_WIDTH: types::GLenum = 0x8D42;
#[allow(dead_code, non_upper_case_globals)] pub const RENDERER: types::GLenum = 0x1F01;
#[allow(dead_code, non_upper_case_globals)] pub const REPEAT: types::GLenum = 0x2901;
#[allow(dead_code, non_upper_case_globals)] pub const REPLACE: types::GLenum = 0x1E01;
#[allow(dead_code, non_upper_case_globals)] pub const RG: types::GLenum = 0x8227;
#[allow(dead_code, non_upper_case_globals)] pub const RG16: types::GLenum = 0x822C;
#[allow(dead_code, non_upper_case_globals)] pub const RG16F: types::GLenum = 0x822F;
#[allow(dead_code, non_upper_case_globals)] pub const RG16I: types::GLenum = 0x8239;
#[allow(dead_code, non_upper_case_globals)] pub const RG16UI: types::GLenum = 0x823A;
#[allow(dead_code, non_upper_case_globals)] pub const RG16_SNORM: types::GLenum = 0x8F99;
#[allow(dead_code, non_upper_case_globals)] pub const RG32F: types::GLenum = 0x8230;
#[allow(dead_code, non_upper_case_globals)] pub const RG32I: types::GLenum = 0x823B;
#[allow(dead_code, non_upper_case_globals)] pub const RG32UI: types::GLenum = 0x823C;
#[allow(dead_code, non_upper_case_globals)] pub const RG8: types::GLenum = 0x822B;
#[allow(dead_code, non_upper_case_globals)] pub const RG8I: types::GLenum = 0x8237;
#[allow(dead_code, non_upper_case_globals)] pub const RG8UI: types::GLenum = 0x8238;
#[allow(dead_code, non_upper_case_globals)] pub const RG8_SNORM: types::GLenum = 0x8F95;
#[allow(dead_code, non_upper_case_globals)] pub const RGB: types::GLenum = 0x1907;
#[allow(dead_code, non_upper_case_globals)] pub const RGB10: types::GLenum = 0x8052;
#[allow(dead_code, non_upper_case_globals)] pub const RGB10_A2: types::GLenum = 0x8059;
#[allow(dead_code, non_upper_case_globals)] pub const RGB10_A2UI: types::GLenum = 0x906F;
#[allow(dead_code, non_upper_case_globals)] pub const RGB12: types::GLenum = 0x8053;
#[allow(dead_code, non_upper_case_globals)] pub const RGB16: types::GLenum = 0x8054;
#[allow(dead_code, non_upper_case_globals)] pub const RGB16F: types::GLenum = 0x881B;
#[allow(dead_code, non_upper_case_globals)] pub const RGB16I: types::GLenum = 0x8D89;
#[allow(dead_code, non_upper_case_globals)] pub const RGB16UI: types::GLenum = 0x8D77;
#[allow(dead_code, non_upper_case_globals)] pub const RGB16_SNORM: types::GLenum = 0x8F9A;
#[allow(dead_code, non_upper_case_globals)] pub const RGB32F: types::GLenum = 0x8815;
#[allow(dead_code, non_upper_case_globals)] pub const RGB32I: types::GLenum = 0x8D83;
#[allow(dead_code, non_upper_case_globals)] pub const RGB32UI: types::GLenum = 0x8D71;
#[allow(dead_code, non_upper_case_globals)] pub const RGB4: types::GLenum = 0x804F;
#[allow(dead_code, non_upper_case_globals)] pub const RGB5: types::GLenum = 0x8050;
#[allow(dead_code, non_upper_case_globals)] pub const RGB5_A1: types::GLenum = 0x8057;
#[allow(dead_code, non_upper_case_globals)] pub const RGB8: types::GLenum = 0x8051;
#[allow(dead_code, non_upper_case_globals)] pub const RGB8I: types::GLenum = 0x8D8F;
#[allow(dead_code, non_upper_case_globals)] pub const RGB8UI: types::GLenum = 0x8D7D;
#[allow(dead_code, non_upper_case_globals)] pub const RGB8_SNORM: types::GLenum = 0x8F96;
#[allow(dead_code, non_upper_case_globals)] pub const RGB9_E5: types::GLenum = 0x8C3D;
#[allow(dead_code, non_upper_case_globals)] pub const RGBA: types::GLenum = 0x1908;
#[allow(dead_code, non_upper_case_globals)] pub const RGBA12: types::GLenum = 0x805A;
#[allow(dead_code, non_upper_case_globals)] pub const RGBA16: types::GLenum = 0x805B;
#[allow(dead_code, non_upper_case_globals)] pub const RGBA16F: types::GLenum = 0x881A;
#[allow(dead_code, non_upper_case_globals)] pub const RGBA16I: types::GLenum = 0x8D88;
#[allow(dead_code, non_upper_case_globals)] pub const RGBA16UI: types::GLenum = 0x8D76;
#[allow(dead_code, non_upper_case_globals)] pub const RGBA16_SNORM: types::GLenum = 0x8F9B;
#[allow(dead_code, non_upper_case_globals)] pub const RGBA2: types::GLenum = 0x8055;
#[allow(dead_code, non_upper_case_globals)] pub const RGBA32F: types::GLenum = 0x8814;
#[allow(dead_code, non_upper_case_globals)] pub const RGBA32I: types::GLenum = 0x8D82;
#[allow(dead_code, non_upper_case_globals)] pub const RGBA32UI: types::GLenum = 0x8D70;
#[allow(dead_code, non_upper_case_globals)] pub const RGBA4: types::GLenum = 0x8056;
#[allow(dead_code, non_upper_case_globals)] pub const RGBA8: types::GLenum = 0x8058;
#[allow(dead_code, non_upper_case_globals)] pub const RGBA8I: types::GLenum = 0x8D8E;
#[allow(dead_code, non_upper_case_globals)] pub const RGBA8UI: types::GLenum = 0x8D7C;
#[allow(dead_code, non_upper_case_globals)] pub const RGBA8_SNORM: types::GLenum = 0x8F97;
#[allow(dead_code, non_upper_case_globals)] pub const RGBA_INTEGER: types::GLenum = 0x8D99;
#[allow(dead_code, non_upper_case_globals)] pub const RGB_INTEGER: types::GLenum = 0x8D98;
#[allow(dead_code, non_upper_case_globals)] pub const RG_INTEGER: types::GLenum = 0x8228;
#[allow(dead_code, non_upper_case_globals)] pub const RIGHT: types::GLenum = 0x0407;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLER_1D: types::GLenum = 0x8B5D;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLER_1D_ARRAY: types::GLenum = 0x8DC0;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLER_1D_ARRAY_SHADOW: types::GLenum = 0x8DC3;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLER_1D_SHADOW: types::GLenum = 0x8B61;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLER_2D: types::GLenum = 0x8B5E;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLER_2D_ARRAY: types::GLenum = 0x8DC1;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLER_2D_ARRAY_SHADOW: types::GLenum = 0x8DC4;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLER_2D_MULTISAMPLE: types::GLenum = 0x9108;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLER_2D_MULTISAMPLE_ARRAY: types::GLenum = 0x910B;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLER_2D_RECT: types::GLenum = 0x8B63;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLER_2D_RECT_SHADOW: types::GLenum = 0x8B64;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLER_2D_SHADOW: types::GLenum = 0x8B62;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLER_3D: types::GLenum = 0x8B5F;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLER_BINDING: types::GLenum = 0x8919;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLER_BUFFER: types::GLenum = 0x8DC2;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLER_CUBE: types::GLenum = 0x8B60;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLER_CUBE_SHADOW: types::GLenum = 0x8DC5;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLES: types::GLenum = 0x80A9;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLES_PASSED: types::GLenum = 0x8914;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLE_ALPHA_TO_COVERAGE: types::GLenum = 0x809E;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLE_ALPHA_TO_ONE: types::GLenum = 0x809F;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLE_BUFFERS: types::GLenum = 0x80A8;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLE_COVERAGE: types::GLenum = 0x80A0;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLE_COVERAGE_INVERT: types::GLenum = 0x80AB;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLE_COVERAGE_VALUE: types::GLenum = 0x80AA;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLE_MASK: types::GLenum = 0x8E51;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLE_MASK_VALUE: types::GLenum = 0x8E52;
#[allow(dead_code, non_upper_case_globals)] pub const SAMPLE_POSITION: types::GLenum = 0x8E50;
#[allow(dead_code, non_upper_case_globals)] pub const SCISSOR_BOX: types::GLenum = 0x0C10;
#[allow(dead_code, non_upper_case_globals)] pub const SCISSOR_TEST: types::GLenum = 0x0C11;
#[allow(dead_code, non_upper_case_globals)] pub const SEPARATE_ATTRIBS: types::GLenum = 0x8C8D;
#[allow(dead_code, non_upper_case_globals)] pub const SET: types::GLenum = 0x150F;
#[allow(dead_code, non_upper_case_globals)] pub const SHADER_SOURCE_LENGTH: types::GLenum = 0x8B88;
#[allow(dead_code, non_upper_case_globals)] pub const SHADER_TYPE: types::GLenum = 0x8B4F;
#[allow(dead_code, non_upper_case_globals)] pub const SHADING_LANGUAGE_VERSION: types::GLenum = 0x8B8C;
#[allow(dead_code, non_upper_case_globals)] pub const SHORT: types::GLenum = 0x1402;
#[allow(dead_code, non_upper_case_globals)] pub const SIGNALED: types::GLenum = 0x9119;
#[allow(dead_code, non_upper_case_globals)] pub const SIGNED_NORMALIZED: types::GLenum = 0x8F9C;
#[allow(dead_code, non_upper_case_globals)] pub const SMOOTH_LINE_WIDTH_GRANULARITY: types::GLenum = 0x0B23;
#[allow(dead_code, non_upper_case_globals)] pub const SMOOTH_LINE_WIDTH_RANGE: types::GLenum = 0x0B22;
#[allow(dead_code, non_upper_case_globals)] pub const SMOOTH_POINT_SIZE_GRANULARITY: types::GLenum = 0x0B13;
#[allow(dead_code, non_upper_case_globals)] pub const SMOOTH_POINT_SIZE_RANGE: types::GLenum = 0x0B12;
#[allow(dead_code, non_upper_case_globals)] pub const SRC1_ALPHA: types::GLenum = 0x8589;
#[allow(dead_code, non_upper_case_globals)] pub const SRC1_COLOR: types::GLenum = 0x88F9;
#[allow(dead_code, non_upper_case_globals)] pub const SRC_ALPHA: types::GLenum = 0x0302;
#[allow(dead_code, non_upper_case_globals)] pub const SRC_ALPHA_SATURATE: types::GLenum = 0x0308;
#[allow(dead_code, non_upper_case_globals)] pub const SRC_COLOR: types::GLenum = 0x0300;
#[allow(dead_code, non_upper_case_globals)] pub const SRGB: types::GLenum = 0x8C40;
#[allow(dead_code, non_upper_case_globals)] pub const SRGB8: types::GLenum = 0x8C41;
#[allow(dead_code, non_upper_case_globals)] pub const SRGB8_ALPHA8: types::GLenum = 0x8C43;
#[allow(dead_code, non_upper_case_globals)] pub const SRGB_ALPHA: types::GLenum = 0x8C42;
#[allow(dead_code, non_upper_case_globals)] pub const STATIC_COPY: types::GLenum = 0x88E6;
#[allow(dead_code, non_upper_case_globals)] pub const STATIC_DRAW: types::GLenum = 0x88E4;
#[allow(dead_code, non_upper_case_globals)] pub const STATIC_READ: types::GLenum = 0x88E5;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL: types::GLenum = 0x1802;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_ATTACHMENT: types::GLenum = 0x8D20;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_BACK_FAIL: types::GLenum = 0x8801;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_BACK_FUNC: types::GLenum = 0x8800;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_BACK_PASS_DEPTH_FAIL: types::GLenum = 0x8802;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_BACK_PASS_DEPTH_PASS: types::GLenum = 0x8803;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_BACK_REF: types::GLenum = 0x8CA3;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_BACK_VALUE_MASK: types::GLenum = 0x8CA4;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_BACK_WRITEMASK: types::GLenum = 0x8CA5;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_BUFFER_BIT: types::GLenum = 0x00000400;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_CLEAR_VALUE: types::GLenum = 0x0B91;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_FAIL: types::GLenum = 0x0B94;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_FUNC: types::GLenum = 0x0B92;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_INDEX: types::GLenum = 0x1901;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_INDEX1: types::GLenum = 0x8D46;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_INDEX16: types::GLenum = 0x8D49;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_INDEX4: types::GLenum = 0x8D47;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_INDEX8: types::GLenum = 0x8D48;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_PASS_DEPTH_FAIL: types::GLenum = 0x0B95;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_PASS_DEPTH_PASS: types::GLenum = 0x0B96;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_REF: types::GLenum = 0x0B97;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_TEST: types::GLenum = 0x0B90;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_VALUE_MASK: types::GLenum = 0x0B93;
#[allow(dead_code, non_upper_case_globals)] pub const STENCIL_WRITEMASK: types::GLenum = 0x0B98;
#[allow(dead_code, non_upper_case_globals)] pub const STEREO: types::GLenum = 0x0C33;
#[allow(dead_code, non_upper_case_globals)] pub const STREAM_COPY: types::GLenum = 0x88E2;
#[allow(dead_code, non_upper_case_globals)] pub const STREAM_DRAW: types::GLenum = 0x88E0;
#[allow(dead_code, non_upper_case_globals)] pub const STREAM_READ: types::GLenum = 0x88E1;
#[allow(dead_code, non_upper_case_globals)] pub const SUBPIXEL_BITS: types::GLenum = 0x0D50;
#[allow(dead_code, non_upper_case_globals)] pub const SYNC_CONDITION: types::GLenum = 0x9113;
#[allow(dead_code, non_upper_case_globals)] pub const SYNC_FENCE: types::GLenum = 0x9116;
#[allow(dead_code, non_upper_case_globals)] pub const SYNC_FLAGS: types::GLenum = 0x9115;
#[allow(dead_code, non_upper_case_globals)] pub const SYNC_FLUSH_COMMANDS_BIT: types::GLenum = 0x00000001;
#[allow(dead_code, non_upper_case_globals)] pub const SYNC_GPU_COMMANDS_COMPLETE: types::GLenum = 0x9117;
#[allow(dead_code, non_upper_case_globals)] pub const SYNC_STATUS: types::GLenum = 0x9114;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE: types::GLenum = 0x1702;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE0: types::GLenum = 0x84C0;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE1: types::GLenum = 0x84C1;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE10: types::GLenum = 0x84CA;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE11: types::GLenum = 0x84CB;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE12: types::GLenum = 0x84CC;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE13: types::GLenum = 0x84CD;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE14: types::GLenum = 0x84CE;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE15: types::GLenum = 0x84CF;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE16: types::GLenum = 0x84D0;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE17: types::GLenum = 0x84D1;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE18: types::GLenum = 0x84D2;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE19: types::GLenum = 0x84D3;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE2: types::GLenum = 0x84C2;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE20: types::GLenum = 0x84D4;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE21: types::GLenum = 0x84D5;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE22: types::GLenum = 0x84D6;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE23: types::GLenum = 0x84D7;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE24: types::GLenum = 0x84D8;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE25: types::GLenum = 0x84D9;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE26: types::GLenum = 0x84DA;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE27: types::GLenum = 0x84DB;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE28: types::GLenum = 0x84DC;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE29: types::GLenum = 0x84DD;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE3: types::GLenum = 0x84C3;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE30: types::GLenum = 0x84DE;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE31: types::GLenum = 0x84DF;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE4: types::GLenum = 0x84C4;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE5: types::GLenum = 0x84C5;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE6: types::GLenum = 0x84C6;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE7: types::GLenum = 0x84C7;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE8: types::GLenum = 0x84C8;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE9: types::GLenum = 0x84C9;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_1D: types::GLenum = 0x0DE0;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_1D_ARRAY: types::GLenum = 0x8C18;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_2D: types::GLenum = 0x0DE1;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_2D_ARRAY: types::GLenum = 0x8C1A;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_2D_MULTISAMPLE: types::GLenum = 0x9100;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_2D_MULTISAMPLE_ARRAY: types::GLenum = 0x9102;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_3D: types::GLenum = 0x806F;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_ALPHA_SIZE: types::GLenum = 0x805F;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_ALPHA_TYPE: types::GLenum = 0x8C13;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_BASE_LEVEL: types::GLenum = 0x813C;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_BINDING_1D: types::GLenum = 0x8068;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_BINDING_1D_ARRAY: types::GLenum = 0x8C1C;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_BINDING_2D: types::GLenum = 0x8069;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_BINDING_2D_ARRAY: types::GLenum = 0x8C1D;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_BINDING_2D_MULTISAMPLE: types::GLenum = 0x9104;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_BINDING_2D_MULTISAMPLE_ARRAY: types::GLenum = 0x9105;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_BINDING_3D: types::GLenum = 0x806A;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_BINDING_BUFFER: types::GLenum = 0x8C2C;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_BINDING_CUBE_MAP: types::GLenum = 0x8514;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_BINDING_RECTANGLE: types::GLenum = 0x84F6;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_BLUE_SIZE: types::GLenum = 0x805E;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_BLUE_TYPE: types::GLenum = 0x8C12;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_BORDER_COLOR: types::GLenum = 0x1004;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_BUFFER: types::GLenum = 0x8C2A;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_BUFFER_DATA_STORE_BINDING: types::GLenum = 0x8C2D;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_COMPARE_FUNC: types::GLenum = 0x884D;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_COMPARE_MODE: types::GLenum = 0x884C;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_COMPRESSED: types::GLenum = 0x86A1;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_COMPRESSED_IMAGE_SIZE: types::GLenum = 0x86A0;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_COMPRESSION_HINT: types::GLenum = 0x84EF;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_CUBE_MAP: types::GLenum = 0x8513;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_CUBE_MAP_NEGATIVE_X: types::GLenum = 0x8516;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_CUBE_MAP_NEGATIVE_Y: types::GLenum = 0x8518;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_CUBE_MAP_NEGATIVE_Z: types::GLenum = 0x851A;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_CUBE_MAP_POSITIVE_X: types::GLenum = 0x8515;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_CUBE_MAP_POSITIVE_Y: types::GLenum = 0x8517;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_CUBE_MAP_POSITIVE_Z: types::GLenum = 0x8519;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_CUBE_MAP_SEAMLESS: types::GLenum = 0x884F;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_DEPTH: types::GLenum = 0x8071;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_DEPTH_SIZE: types::GLenum = 0x884A;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_DEPTH_TYPE: types::GLenum = 0x8C16;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_FIXED_SAMPLE_LOCATIONS: types::GLenum = 0x9107;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_GREEN_SIZE: types::GLenum = 0x805D;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_GREEN_TYPE: types::GLenum = 0x8C11;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_HEIGHT: types::GLenum = 0x1001;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_INTERNAL_FORMAT: types::GLenum = 0x1003;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_LOD_BIAS: types::GLenum = 0x8501;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_MAG_FILTER: types::GLenum = 0x2800;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_MAX_LEVEL: types::GLenum = 0x813D;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_MAX_LOD: types::GLenum = 0x813B;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_MIN_FILTER: types::GLenum = 0x2801;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_MIN_LOD: types::GLenum = 0x813A;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_RECTANGLE: types::GLenum = 0x84F5;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_RED_SIZE: types::GLenum = 0x805C;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_RED_TYPE: types::GLenum = 0x8C10;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_SAMPLES: types::GLenum = 0x9106;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_SHARED_SIZE: types::GLenum = 0x8C3F;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_STENCIL_SIZE: types::GLenum = 0x88F1;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_SWIZZLE_A: types::GLenum = 0x8E45;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_SWIZZLE_B: types::GLenum = 0x8E44;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_SWIZZLE_G: types::GLenum = 0x8E43;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_SWIZZLE_R: types::GLenum = 0x8E42;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_SWIZZLE_RGBA: types::GLenum = 0x8E46;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_WIDTH: types::GLenum = 0x1000;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_WRAP_R: types::GLenum = 0x8072;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_WRAP_S: types::GLenum = 0x2802;
#[allow(dead_code, non_upper_case_globals)] pub const TEXTURE_WRAP_T: types::GLenum = 0x2803;
#[allow(dead_code, non_upper_case_globals)] pub const TIMEOUT_EXPIRED: types::GLenum = 0x911B;
#[allow(dead_code, non_upper_case_globals)] pub const TIMEOUT_IGNORED: types::GLuint64 = 0xFFFFFFFFFFFFFFFF;
#[allow(dead_code, non_upper_case_globals)] pub const TIMESTAMP: types::GLenum = 0x8E28;
#[allow(dead_code, non_upper_case_globals)] pub const TIME_ELAPSED: types::GLenum = 0x88BF;
#[allow(dead_code, non_upper_case_globals)] pub const TRANSFORM_FEEDBACK_BUFFER: types::GLenum = 0x8C8E;
#[allow(dead_code, non_upper_case_globals)] pub const TRANSFORM_FEEDBACK_BUFFER_BINDING: types::GLenum = 0x8C8F;
#[allow(dead_code, non_upper_case_globals)] pub const TRANSFORM_FEEDBACK_BUFFER_MODE: types::GLenum = 0x8C7F;
#[allow(dead_code, non_upper_case_globals)] pub const TRANSFORM_FEEDBACK_BUFFER_SIZE: types::GLenum = 0x8C85;
#[allow(dead_code, non_upper_case_globals)] pub const TRANSFORM_FEEDBACK_BUFFER_START: types::GLenum = 0x8C84;
#[allow(dead_code, non_upper_case_globals)] pub const TRANSFORM_FEEDBACK_PRIMITIVES_WRITTEN: types::GLenum = 0x8C88;
#[allow(dead_code, non_upper_case_globals)] pub const TRANSFORM_FEEDBACK_VARYINGS: types::GLenum = 0x8C83;
#[allow(dead_code, non_upper_case_globals)] pub const TRANSFORM_FEEDBACK_VARYING_MAX_LENGTH: types::GLenum = 0x8C76;
#[allow(dead_code, non_upper_case_globals)] pub const TRIANGLES: types::GLenum = 0x0004;
#[allow(dead_code, non_upper_case_globals)] pub const TRIANGLES_ADJACENCY: types::GLenum = 0x000C;
#[allow(dead_code, non_upper_case_globals)] pub const TRIANGLE_FAN: types::GLenum = 0x0006;
#[allow(dead_code, non_upper_case_globals)] pub const TRIANGLE_STRIP: types::GLenum = 0x0005;
#[allow(dead_code, non_upper_case_globals)] pub const TRIANGLE_STRIP_ADJACENCY: types::GLenum = 0x000D;
#[allow(dead_code, non_upper_case_globals)] pub const TRUE: types::GLboolean = 1;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_ARRAY_STRIDE: types::GLenum = 0x8A3C;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_BLOCK_ACTIVE_UNIFORMS: types::GLenum = 0x8A42;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_BLOCK_ACTIVE_UNIFORM_INDICES: types::GLenum = 0x8A43;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_BLOCK_BINDING: types::GLenum = 0x8A3F;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_BLOCK_DATA_SIZE: types::GLenum = 0x8A40;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_BLOCK_INDEX: types::GLenum = 0x8A3A;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_BLOCK_NAME_LENGTH: types::GLenum = 0x8A41;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_BLOCK_REFERENCED_BY_FRAGMENT_SHADER: types::GLenum = 0x8A46;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_BLOCK_REFERENCED_BY_GEOMETRY_SHADER: types::GLenum = 0x8A45;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_BLOCK_REFERENCED_BY_VERTEX_SHADER: types::GLenum = 0x8A44;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_BUFFER: types::GLenum = 0x8A11;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_BUFFER_BINDING: types::GLenum = 0x8A28;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_BUFFER_OFFSET_ALIGNMENT: types::GLenum = 0x8A34;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_BUFFER_SIZE: types::GLenum = 0x8A2A;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_BUFFER_START: types::GLenum = 0x8A29;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_IS_ROW_MAJOR: types::GLenum = 0x8A3E;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_MATRIX_STRIDE: types::GLenum = 0x8A3D;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_NAME_LENGTH: types::GLenum = 0x8A39;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_OFFSET: types::GLenum = 0x8A3B;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_SIZE: types::GLenum = 0x8A38;
#[allow(dead_code, non_upper_case_globals)] pub const UNIFORM_TYPE: types::GLenum = 0x8A37;
#[allow(dead_code, non_upper_case_globals)] pub const UNPACK_ALIGNMENT: types::GLenum = 0x0CF5;
#[allow(dead_code, non_upper_case_globals)] pub const UNPACK_IMAGE_HEIGHT: types::GLenum = 0x806E;
#[allow(dead_code, non_upper_case_globals)] pub const UNPACK_LSB_FIRST: types::GLenum = 0x0CF1;
#[allow(dead_code, non_upper_case_globals)] pub const UNPACK_ROW_LENGTH: types::GLenum = 0x0CF2;
#[allow(dead_code, non_upper_case_globals)] pub const UNPACK_SKIP_IMAGES: types::GLenum = 0x806D;
#[allow(dead_code, non_upper_case_globals)] pub const UNPACK_SKIP_PIXELS: types::GLenum = 0x0CF4;
#[allow(dead_code, non_upper_case_globals)] pub const UNPACK_SKIP_ROWS: types::GLenum = 0x0CF3;
#[allow(dead_code, non_upper_case_globals)] pub const UNPACK_SWAP_BYTES: types::GLenum = 0x0CF0;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNALED: types::GLenum = 0x9118;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_BYTE: types::GLenum = 0x1401;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_BYTE_2_3_3_REV: types::GLenum = 0x8362;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_BYTE_3_3_2: types::GLenum = 0x8032;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT: types::GLenum = 0x1405;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_10F_11F_11F_REV: types::GLenum = 0x8C3B;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_10_10_10_2: types::GLenum = 0x8036;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_24_8: types::GLenum = 0x84FA;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_2_10_10_10_REV: types::GLenum = 0x8368;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_5_9_9_9_REV: types::GLenum = 0x8C3E;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_8_8_8_8: types::GLenum = 0x8035;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_8_8_8_8_REV: types::GLenum = 0x8367;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_SAMPLER_1D: types::GLenum = 0x8DD1;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_SAMPLER_1D_ARRAY: types::GLenum = 0x8DD6;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_SAMPLER_2D: types::GLenum = 0x8DD2;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_SAMPLER_2D_ARRAY: types::GLenum = 0x8DD7;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_SAMPLER_2D_MULTISAMPLE: types::GLenum = 0x910A;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_SAMPLER_2D_MULTISAMPLE_ARRAY: types::GLenum = 0x910D;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_SAMPLER_2D_RECT: types::GLenum = 0x8DD5;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_SAMPLER_3D: types::GLenum = 0x8DD3;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_SAMPLER_BUFFER: types::GLenum = 0x8DD8;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_SAMPLER_CUBE: types::GLenum = 0x8DD4;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_VEC2: types::GLenum = 0x8DC6;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_VEC3: types::GLenum = 0x8DC7;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_INT_VEC4: types::GLenum = 0x8DC8;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_NORMALIZED: types::GLenum = 0x8C17;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_SHORT: types::GLenum = 0x1403;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_SHORT_1_5_5_5_REV: types::GLenum = 0x8366;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_SHORT_4_4_4_4: types::GLenum = 0x8033;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_SHORT_4_4_4_4_REV: types::GLenum = 0x8365;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_SHORT_5_5_5_1: types::GLenum = 0x8034;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_SHORT_5_6_5: types::GLenum = 0x8363;
#[allow(dead_code, non_upper_case_globals)] pub const UNSIGNED_SHORT_5_6_5_REV: types::GLenum = 0x8364;
#[allow(dead_code, non_upper_case_globals)] pub const UPPER_LEFT: types::GLenum = 0x8CA2;
#[allow(dead_code, non_upper_case_globals)] pub const VALIDATE_STATUS: types::GLenum = 0x8B83;
#[allow(dead_code, non_upper_case_globals)] pub const VENDOR: types::GLenum = 0x1F00;
#[allow(dead_code, non_upper_case_globals)] pub const VERSION: types::GLenum = 0x1F02;
#[allow(dead_code, non_upper_case_globals)] pub const VERTEX_ARRAY_BINDING: types::GLenum = 0x85B5;
#[allow(dead_code, non_upper_case_globals)] pub const VERTEX_ATTRIB_ARRAY_BUFFER_BINDING: types::GLenum = 0x889F;
#[allow(dead_code, non_upper_case_globals)] pub const VERTEX_ATTRIB_ARRAY_DIVISOR: types::GLenum = 0x88FE;
#[allow(dead_code, non_upper_case_globals)] pub const VERTEX_ATTRIB_ARRAY_ENABLED: types::GLenum = 0x8622;
#[allow(dead_code, non_upper_case_globals)] pub const VERTEX_ATTRIB_ARRAY_INTEGER: types::GLenum = 0x88FD;
#[allow(dead_code, non_upper_case_globals)] pub const VERTEX_ATTRIB_ARRAY_NORMALIZED: types::GLenum = 0x886A;
#[allow(dead_code, non_upper_case_globals)] pub const VERTEX_ATTRIB_ARRAY_POINTER: types::GLenum = 0x8645;
#[allow(dead_code, non_upper_case_globals)] pub const VERTEX_ATTRIB_ARRAY_SIZE: types::GLenum = 0x8623;
#[allow(dead_code, non_upper_case_globals)] pub const VERTEX_ATTRIB_ARRAY_STRIDE: types::GLenum = 0x8624;
#[allow(dead_code, non_upper_case_globals)] pub const VERTEX_ATTRIB_ARRAY_TYPE: types::GLenum = 0x8625;
#[allow(dead_code, non_upper_case_globals)] pub const VERTEX_PROGRAM_POINT_SIZE: types::GLenum = 0x8642;
#[allow(dead_code, non_upper_case_globals)] pub const VERTEX_SHADER: types::GLenum = 0x8B31;
#[allow(dead_code, non_upper_case_globals)] pub const VIEWPORT: types::GLenum = 0x0BA2;
#[allow(dead_code, non_upper_case_globals)] pub const WAIT_FAILED: types::GLenum = 0x911D;
#[allow(dead_code, non_upper_case_globals)] pub const WRITE_ONLY: types::GLenum = 0x88B9;
#[allow(dead_code, non_upper_case_globals)] pub const XOR: types::GLenum = 0x1506;
#[allow(dead_code, non_upper_case_globals)] pub const ZERO: types::GLenum = 0;
/// Fallbacks: ActiveTextureARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ActiveTexture(texture: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> ()>(storage::ActiveTexture.f)(texture) }
/// Fallbacks: AttachObjectARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn AttachShader(program: types::GLuint, shader: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLuint) -> ()>(storage::AttachShader.f)(program, shader) }
/// Fallbacks: BeginConditionalRenderNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BeginConditionalRender(id: types::GLuint, mode: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum) -> ()>(storage::BeginConditionalRender.f)(id, mode) }
/// Fallbacks: BeginQueryARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BeginQuery(target: types::GLenum, id: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::BeginQuery.f)(target, id) }
/// Fallbacks: BeginTransformFeedbackEXT, BeginTransformFeedbackNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BeginTransformFeedback(primitiveMode: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> ()>(storage::BeginTransformFeedback.f)(primitiveMode) }
/// Fallbacks: BindAttribLocationARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BindAttribLocation(program: types::GLuint, index: types::GLuint, name: *const types::GLchar) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLuint, *const types::GLchar) -> ()>(storage::BindAttribLocation.f)(program, index, name) }
/// Fallbacks: BindBufferARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BindBuffer(target: types::GLenum, buffer: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::BindBuffer.f)(target, buffer) }
/// Fallbacks: BindBufferBaseEXT, BindBufferBaseNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BindBufferBase(target: types::GLenum, index: types::GLuint, buffer: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint, types::GLuint) -> ()>(storage::BindBufferBase.f)(target, index, buffer) }
/// Fallbacks: BindBufferRangeEXT, BindBufferRangeNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BindBufferRange(target: types::GLenum, index: types::GLuint, buffer: types::GLuint, offset: types::GLintptr, size: types::GLsizeiptr) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint, types::GLuint, types::GLintptr, types::GLsizeiptr) -> ()>(storage::BindBufferRange.f)(target, index, buffer, offset, size) }
/// Fallbacks: BindFragDataLocationEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BindFragDataLocation(program: types::GLuint, color: types::GLuint, name: *const types::GLchar) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLuint, *const types::GLchar) -> ()>(storage::BindFragDataLocation.f)(program, color, name) }
/// Fallbacks: BindFragDataLocationIndexedEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BindFragDataLocationIndexed(program: types::GLuint, colorNumber: types::GLuint, index: types::GLuint, name: *const types::GLchar) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLuint, types::GLuint, *const types::GLchar) -> ()>(storage::BindFragDataLocationIndexed.f)(program, colorNumber, index, name) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BindFramebuffer(target: types::GLenum, framebuffer: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::BindFramebuffer.f)(target, framebuffer) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BindRenderbuffer(target: types::GLenum, renderbuffer: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::BindRenderbuffer.f)(target, renderbuffer) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BindSampler(unit: types::GLuint, sampler: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLuint) -> ()>(storage::BindSampler.f)(unit, sampler) }
/// Fallbacks: BindTextureEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BindTexture(target: types::GLenum, texture: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::BindTexture.f)(target, texture) }
/// Fallbacks: BindVertexArrayOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BindVertexArray(array: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> ()>(storage::BindVertexArray.f)(array) }
/// Fallbacks: BlendColorEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BlendColor(red: types::GLfloat, green: types::GLfloat, blue: types::GLfloat, alpha: types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLfloat, types::GLfloat, types::GLfloat, types::GLfloat) -> ()>(storage::BlendColor.f)(red, green, blue, alpha) }
/// Fallbacks: BlendEquationEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BlendEquation(mode: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> ()>(storage::BlendEquation.f)(mode) }
/// Fallbacks: BlendEquationSeparateEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BlendEquationSeparate(modeRGB: types::GLenum, modeAlpha: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum) -> ()>(storage::BlendEquationSeparate.f)(modeRGB, modeAlpha) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BlendFunc(sfactor: types::GLenum, dfactor: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum) -> ()>(storage::BlendFunc.f)(sfactor, dfactor) }
/// Fallbacks: BlendFuncSeparateEXT, BlendFuncSeparateINGR
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BlendFuncSeparate(sfactorRGB: types::GLenum, dfactorRGB: types::GLenum, sfactorAlpha: types::GLenum, dfactorAlpha: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLenum, types::GLenum) -> ()>(storage::BlendFuncSeparate.f)(sfactorRGB, dfactorRGB, sfactorAlpha, dfactorAlpha) }
/// Fallbacks: BlitFramebufferEXT, BlitFramebufferNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BlitFramebuffer(srcX0: types::GLint, srcY0: types::GLint, srcX1: types::GLint, srcY1: types::GLint, dstX0: types::GLint, dstY0: types::GLint, dstX1: types::GLint, dstY1: types::GLint, mask: types::GLbitfield, filter: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLint, types::GLint, types::GLint, types::GLint, types::GLint, types::GLint, types::GLint, types::GLbitfield, types::GLenum) -> ()>(storage::BlitFramebuffer.f)(srcX0, srcY0, srcX1, srcY1, dstX0, dstY0, dstX1, dstY1, mask, filter) }
/// Fallbacks: BufferDataARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BufferData(target: types::GLenum, size: types::GLsizeiptr, data: *const __gl_imports::raw::c_void, usage: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLsizeiptr, *const __gl_imports::raw::c_void, types::GLenum) -> ()>(storage::BufferData.f)(target, size, data, usage) }
/// Fallbacks: BufferSubDataARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn BufferSubData(target: types::GLenum, offset: types::GLintptr, size: types::GLsizeiptr, data: *const __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLintptr, types::GLsizeiptr, *const __gl_imports::raw::c_void) -> ()>(storage::BufferSubData.f)(target, offset, size, data) }
/// Fallbacks: CheckFramebufferStatusEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn CheckFramebufferStatus(target: types::GLenum) -> types::GLenum { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> types::GLenum>(storage::CheckFramebufferStatus.f)(target) }
/// Fallbacks: ClampColorARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ClampColor(target: types::GLenum, clamp: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum) -> ()>(storage::ClampColor.f)(target, clamp) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Clear(mask: types::GLbitfield) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLbitfield) -> ()>(storage::Clear.f)(mask) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ClearBufferfi(buffer: types::GLenum, drawbuffer: types::GLint, depth: types::GLfloat, stencil: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLfloat, types::GLint) -> ()>(storage::ClearBufferfi.f)(buffer, drawbuffer, depth, stencil) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ClearBufferfv(buffer: types::GLenum, drawbuffer: types::GLint, value: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, *const types::GLfloat) -> ()>(storage::ClearBufferfv.f)(buffer, drawbuffer, value) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ClearBufferiv(buffer: types::GLenum, drawbuffer: types::GLint, value: *const types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, *const types::GLint) -> ()>(storage::ClearBufferiv.f)(buffer, drawbuffer, value) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ClearBufferuiv(buffer: types::GLenum, drawbuffer: types::GLint, value: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, *const types::GLuint) -> ()>(storage::ClearBufferuiv.f)(buffer, drawbuffer, value) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ClearColor(red: types::GLfloat, green: types::GLfloat, blue: types::GLfloat, alpha: types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLfloat, types::GLfloat, types::GLfloat, types::GLfloat) -> ()>(storage::ClearColor.f)(red, green, blue, alpha) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ClearDepth(depth: types::GLdouble) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLdouble) -> ()>(storage::ClearDepth.f)(depth) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ClearStencil(s: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint) -> ()>(storage::ClearStencil.f)(s) }
/// Fallbacks: ClientWaitSyncAPPLE
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ClientWaitSync(sync: types::GLsync, flags: types::GLbitfield, timeout: types::GLuint64) -> types::GLenum { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsync, types::GLbitfield, types::GLuint64) -> types::GLenum>(storage::ClientWaitSync.f)(sync, flags, timeout) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ColorMask(red: types::GLboolean, green: types::GLboolean, blue: types::GLboolean, alpha: types::GLboolean) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLboolean, types::GLboolean, types::GLboolean, types::GLboolean) -> ()>(storage::ColorMask.f)(red, green, blue, alpha) }
/// Fallbacks: ColorMaskIndexedEXT, ColorMaskiEXT, ColorMaskiOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ColorMaski(index: types::GLuint, r: types::GLboolean, g: types::GLboolean, b: types::GLboolean, a: types::GLboolean) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLboolean, types::GLboolean, types::GLboolean, types::GLboolean) -> ()>(storage::ColorMaski.f)(index, r, g, b, a) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ColorP3ui(type_: types::GLenum, color: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::ColorP3ui.f)(type_, color) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ColorP3uiv(type_: types::GLenum, color: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *const types::GLuint) -> ()>(storage::ColorP3uiv.f)(type_, color) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ColorP4ui(type_: types::GLenum, color: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::ColorP4ui.f)(type_, color) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ColorP4uiv(type_: types::GLenum, color: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *const types::GLuint) -> ()>(storage::ColorP4uiv.f)(type_, color) }
/// Fallbacks: CompileShaderARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn CompileShader(shader: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> ()>(storage::CompileShader.f)(shader) }
/// Fallbacks: CompressedTexImage1DARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn CompressedTexImage1D(target: types::GLenum, level: types::GLint, internalformat: types::GLenum, width: types::GLsizei, border: types::GLint, imageSize: types::GLsizei, data: *const __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLenum, types::GLsizei, types::GLint, types::GLsizei, *const __gl_imports::raw::c_void) -> ()>(storage::CompressedTexImage1D.f)(target, level, internalformat, width, border, imageSize, data) }
/// Fallbacks: CompressedTexImage2DARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn CompressedTexImage2D(target: types::GLenum, level: types::GLint, internalformat: types::GLenum, width: types::GLsizei, height: types::GLsizei, border: types::GLint, imageSize: types::GLsizei, data: *const __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLenum, types::GLsizei, types::GLsizei, types::GLint, types::GLsizei, *const __gl_imports::raw::c_void) -> ()>(storage::CompressedTexImage2D.f)(target, level, internalformat, width, height, border, imageSize, data) }
/// Fallbacks: CompressedTexImage3DARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn CompressedTexImage3D(target: types::GLenum, level: types::GLint, internalformat: types::GLenum, width: types::GLsizei, height: types::GLsizei, depth: types::GLsizei, border: types::GLint, imageSize: types::GLsizei, data: *const __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLenum, types::GLsizei, types::GLsizei, types::GLsizei, types::GLint, types::GLsizei, *const __gl_imports::raw::c_void) -> ()>(storage::CompressedTexImage3D.f)(target, level, internalformat, width, height, depth, border, imageSize, data) }
/// Fallbacks: CompressedTexSubImage1DARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn CompressedTexSubImage1D(target: types::GLenum, level: types::GLint, xoffset: types::GLint, width: types::GLsizei, format: types::GLenum, imageSize: types::GLsizei, data: *const __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLint, types::GLsizei, types::GLenum, types::GLsizei, *const __gl_imports::raw::c_void) -> ()>(storage::CompressedTexSubImage1D.f)(target, level, xoffset, width, format, imageSize, data) }
/// Fallbacks: CompressedTexSubImage2DARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn CompressedTexSubImage2D(target: types::GLenum, level: types::GLint, xoffset: types::GLint, yoffset: types::GLint, width: types::GLsizei, height: types::GLsizei, format: types::GLenum, imageSize: types::GLsizei, data: *const __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLint, types::GLint, types::GLsizei, types::GLsizei, types::GLenum, types::GLsizei, *const __gl_imports::raw::c_void) -> ()>(storage::CompressedTexSubImage2D.f)(target, level, xoffset, yoffset, width, height, format, imageSize, data) }
/// Fallbacks: CompressedTexSubImage3DARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn CompressedTexSubImage3D(target: types::GLenum, level: types::GLint, xoffset: types::GLint, yoffset: types::GLint, zoffset: types::GLint, width: types::GLsizei, height: types::GLsizei, depth: types::GLsizei, format: types::GLenum, imageSize: types::GLsizei, data: *const __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLint, types::GLint, types::GLint, types::GLsizei, types::GLsizei, types::GLsizei, types::GLenum, types::GLsizei, *const __gl_imports::raw::c_void) -> ()>(storage::CompressedTexSubImage3D.f)(target, level, xoffset, yoffset, zoffset, width, height, depth, format, imageSize, data) }
/// Fallbacks: CopyBufferSubDataNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn CopyBufferSubData(readTarget: types::GLenum, writeTarget: types::GLenum, readOffset: types::GLintptr, writeOffset: types::GLintptr, size: types::GLsizeiptr) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLintptr, types::GLintptr, types::GLsizeiptr) -> ()>(storage::CopyBufferSubData.f)(readTarget, writeTarget, readOffset, writeOffset, size) }
/// Fallbacks: CopyTexImage1DEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn CopyTexImage1D(target: types::GLenum, level: types::GLint, internalformat: types::GLenum, x: types::GLint, y: types::GLint, width: types::GLsizei, border: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLenum, types::GLint, types::GLint, types::GLsizei, types::GLint) -> ()>(storage::CopyTexImage1D.f)(target, level, internalformat, x, y, width, border) }
/// Fallbacks: CopyTexImage2DEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn CopyTexImage2D(target: types::GLenum, level: types::GLint, internalformat: types::GLenum, x: types::GLint, y: types::GLint, width: types::GLsizei, height: types::GLsizei, border: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLenum, types::GLint, types::GLint, types::GLsizei, types::GLsizei, types::GLint) -> ()>(storage::CopyTexImage2D.f)(target, level, internalformat, x, y, width, height, border) }
/// Fallbacks: CopyTexSubImage1DEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn CopyTexSubImage1D(target: types::GLenum, level: types::GLint, xoffset: types::GLint, x: types::GLint, y: types::GLint, width: types::GLsizei) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLint, types::GLint, types::GLint, types::GLsizei) -> ()>(storage::CopyTexSubImage1D.f)(target, level, xoffset, x, y, width) }
/// Fallbacks: CopyTexSubImage2DEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn CopyTexSubImage2D(target: types::GLenum, level: types::GLint, xoffset: types::GLint, yoffset: types::GLint, x: types::GLint, y: types::GLint, width: types::GLsizei, height: types::GLsizei) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLint, types::GLint, types::GLint, types::GLint, types::GLsizei, types::GLsizei) -> ()>(storage::CopyTexSubImage2D.f)(target, level, xoffset, yoffset, x, y, width, height) }
/// Fallbacks: CopyTexSubImage3DEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn CopyTexSubImage3D(target: types::GLenum, level: types::GLint, xoffset: types::GLint, yoffset: types::GLint, zoffset: types::GLint, x: types::GLint, y: types::GLint, width: types::GLsizei, height: types::GLsizei) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLint, types::GLint, types::GLint, types::GLint, types::GLint, types::GLsizei, types::GLsizei) -> ()>(storage::CopyTexSubImage3D.f)(target, level, xoffset, yoffset, zoffset, x, y, width, height) }
/// Fallbacks: CreateProgramObjectARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn CreateProgram() -> types::GLuint { __gl_imports::mem::transmute::<_, extern "system" fn() -> types::GLuint>(storage::CreateProgram.f)() }
/// Fallbacks: CreateShaderObjectARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn CreateShader(type_: types::GLenum) -> types::GLuint { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> types::GLuint>(storage::CreateShader.f)(type_) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn CullFace(mode: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> ()>(storage::CullFace.f)(mode) }
/// Fallbacks: DeleteBuffersARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DeleteBuffers(n: types::GLsizei, buffers: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsizei, *const types::GLuint) -> ()>(storage::DeleteBuffers.f)(n, buffers) }
/// Fallbacks: DeleteFramebuffersEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DeleteFramebuffers(n: types::GLsizei, framebuffers: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsizei, *const types::GLuint) -> ()>(storage::DeleteFramebuffers.f)(n, framebuffers) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DeleteProgram(program: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> ()>(storage::DeleteProgram.f)(program) }
/// Fallbacks: DeleteQueriesARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DeleteQueries(n: types::GLsizei, ids: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsizei, *const types::GLuint) -> ()>(storage::DeleteQueries.f)(n, ids) }
/// Fallbacks: DeleteRenderbuffersEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DeleteRenderbuffers(n: types::GLsizei, renderbuffers: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsizei, *const types::GLuint) -> ()>(storage::DeleteRenderbuffers.f)(n, renderbuffers) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DeleteSamplers(count: types::GLsizei, samplers: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsizei, *const types::GLuint) -> ()>(storage::DeleteSamplers.f)(count, samplers) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DeleteShader(shader: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> ()>(storage::DeleteShader.f)(shader) }
/// Fallbacks: DeleteSyncAPPLE
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DeleteSync(sync: types::GLsync) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsync) -> ()>(storage::DeleteSync.f)(sync) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DeleteTextures(n: types::GLsizei, textures: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsizei, *const types::GLuint) -> ()>(storage::DeleteTextures.f)(n, textures) }
/// Fallbacks: DeleteVertexArraysAPPLE, DeleteVertexArraysOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DeleteVertexArrays(n: types::GLsizei, arrays: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsizei, *const types::GLuint) -> ()>(storage::DeleteVertexArrays.f)(n, arrays) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DepthFunc(func: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> ()>(storage::DepthFunc.f)(func) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DepthMask(flag: types::GLboolean) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLboolean) -> ()>(storage::DepthMask.f)(flag) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DepthRange(n: types::GLdouble, f: types::GLdouble) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLdouble, types::GLdouble) -> ()>(storage::DepthRange.f)(n, f) }
/// Fallbacks: DetachObjectARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DetachShader(program: types::GLuint, shader: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLuint) -> ()>(storage::DetachShader.f)(program, shader) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Disable(cap: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> ()>(storage::Disable.f)(cap) }
/// Fallbacks: DisableVertexAttribArrayARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DisableVertexAttribArray(index: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> ()>(storage::DisableVertexAttribArray.f)(index) }
/// Fallbacks: DisableIndexedEXT, DisableiEXT, DisableiNV, DisableiOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Disablei(target: types::GLenum, index: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::Disablei.f)(target, index) }
/// Fallbacks: DrawArraysEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DrawArrays(mode: types::GLenum, first: types::GLint, count: types::GLsizei) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLsizei) -> ()>(storage::DrawArrays.f)(mode, first, count) }
/// Fallbacks: DrawArraysInstancedANGLE, DrawArraysInstancedARB, DrawArraysInstancedEXT, DrawArraysInstancedNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DrawArraysInstanced(mode: types::GLenum, first: types::GLint, count: types::GLsizei, instancecount: types::GLsizei) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLsizei, types::GLsizei) -> ()>(storage::DrawArraysInstanced.f)(mode, first, count, instancecount) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DrawBuffer(buf: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> ()>(storage::DrawBuffer.f)(buf) }
/// Fallbacks: DrawBuffersARB, DrawBuffersATI, DrawBuffersEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DrawBuffers(n: types::GLsizei, bufs: *const types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsizei, *const types::GLenum) -> ()>(storage::DrawBuffers.f)(n, bufs) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DrawElements(mode: types::GLenum, count: types::GLsizei, type_: types::GLenum, indices: *const __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLsizei, types::GLenum, *const __gl_imports::raw::c_void) -> ()>(storage::DrawElements.f)(mode, count, type_, indices) }
/// Fallbacks: DrawElementsBaseVertexEXT, DrawElementsBaseVertexOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DrawElementsBaseVertex(mode: types::GLenum, count: types::GLsizei, type_: types::GLenum, indices: *const __gl_imports::raw::c_void, basevertex: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLsizei, types::GLenum, *const __gl_imports::raw::c_void, types::GLint) -> ()>(storage::DrawElementsBaseVertex.f)(mode, count, type_, indices, basevertex) }
/// Fallbacks: DrawElementsInstancedANGLE, DrawElementsInstancedARB, DrawElementsInstancedEXT, DrawElementsInstancedNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DrawElementsInstanced(mode: types::GLenum, count: types::GLsizei, type_: types::GLenum, indices: *const __gl_imports::raw::c_void, instancecount: types::GLsizei) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLsizei, types::GLenum, *const __gl_imports::raw::c_void, types::GLsizei) -> ()>(storage::DrawElementsInstanced.f)(mode, count, type_, indices, instancecount) }
/// Fallbacks: DrawElementsInstancedBaseVertexEXT, DrawElementsInstancedBaseVertexOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DrawElementsInstancedBaseVertex(mode: types::GLenum, count: types::GLsizei, type_: types::GLenum, indices: *const __gl_imports::raw::c_void, instancecount: types::GLsizei, basevertex: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLsizei, types::GLenum, *const __gl_imports::raw::c_void, types::GLsizei, types::GLint) -> ()>(storage::DrawElementsInstancedBaseVertex.f)(mode, count, type_, indices, instancecount, basevertex) }
/// Fallbacks: DrawRangeElementsEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DrawRangeElements(mode: types::GLenum, start: types::GLuint, end: types::GLuint, count: types::GLsizei, type_: types::GLenum, indices: *const __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint, types::GLuint, types::GLsizei, types::GLenum, *const __gl_imports::raw::c_void) -> ()>(storage::DrawRangeElements.f)(mode, start, end, count, type_, indices) }
/// Fallbacks: DrawRangeElementsBaseVertexEXT, DrawRangeElementsBaseVertexOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn DrawRangeElementsBaseVertex(mode: types::GLenum, start: types::GLuint, end: types::GLuint, count: types::GLsizei, type_: types::GLenum, indices: *const __gl_imports::raw::c_void, basevertex: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint, types::GLuint, types::GLsizei, types::GLenum, *const __gl_imports::raw::c_void, types::GLint) -> ()>(storage::DrawRangeElementsBaseVertex.f)(mode, start, end, count, type_, indices, basevertex) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Enable(cap: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> ()>(storage::Enable.f)(cap) }
/// Fallbacks: EnableVertexAttribArrayARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn EnableVertexAttribArray(index: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> ()>(storage::EnableVertexAttribArray.f)(index) }
/// Fallbacks: EnableIndexedEXT, EnableiEXT, EnableiNV, EnableiOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Enablei(target: types::GLenum, index: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::Enablei.f)(target, index) }
/// Fallbacks: EndConditionalRenderNV, EndConditionalRenderNVX
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn EndConditionalRender() -> () { __gl_imports::mem::transmute::<_, extern "system" fn() -> ()>(storage::EndConditionalRender.f)() }
/// Fallbacks: EndQueryARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn EndQuery(target: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> ()>(storage::EndQuery.f)(target) }
/// Fallbacks: EndTransformFeedbackEXT, EndTransformFeedbackNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn EndTransformFeedback() -> () { __gl_imports::mem::transmute::<_, extern "system" fn() -> ()>(storage::EndTransformFeedback.f)() }
/// Fallbacks: FenceSyncAPPLE
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn FenceSync(condition: types::GLenum, flags: types::GLbitfield) -> types::GLsync { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLbitfield) -> types::GLsync>(storage::FenceSync.f)(condition, flags) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Finish() -> () { __gl_imports::mem::transmute::<_, extern "system" fn() -> ()>(storage::Finish.f)() }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Flush() -> () { __gl_imports::mem::transmute::<_, extern "system" fn() -> ()>(storage::Flush.f)() }
/// Fallbacks: FlushMappedBufferRangeAPPLE, FlushMappedBufferRangeEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn FlushMappedBufferRange(target: types::GLenum, offset: types::GLintptr, length: types::GLsizeiptr) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLintptr, types::GLsizeiptr) -> ()>(storage::FlushMappedBufferRange.f)(target, offset, length) }
/// Fallbacks: FramebufferRenderbufferEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn FramebufferRenderbuffer(target: types::GLenum, attachment: types::GLenum, renderbuffertarget: types::GLenum, renderbuffer: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLenum, types::GLuint) -> ()>(storage::FramebufferRenderbuffer.f)(target, attachment, renderbuffertarget, renderbuffer) }
/// Fallbacks: FramebufferTextureARB, FramebufferTextureEXT, FramebufferTextureOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn FramebufferTexture(target: types::GLenum, attachment: types::GLenum, texture: types::GLuint, level: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLuint, types::GLint) -> ()>(storage::FramebufferTexture.f)(target, attachment, texture, level) }
/// Fallbacks: FramebufferTexture1DEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn FramebufferTexture1D(target: types::GLenum, attachment: types::GLenum, textarget: types::GLenum, texture: types::GLuint, level: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLenum, types::GLuint, types::GLint) -> ()>(storage::FramebufferTexture1D.f)(target, attachment, textarget, texture, level) }
/// Fallbacks: FramebufferTexture2DEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn FramebufferTexture2D(target: types::GLenum, attachment: types::GLenum, textarget: types::GLenum, texture: types::GLuint, level: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLenum, types::GLuint, types::GLint) -> ()>(storage::FramebufferTexture2D.f)(target, attachment, textarget, texture, level) }
/// Fallbacks: FramebufferTexture3DEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn FramebufferTexture3D(target: types::GLenum, attachment: types::GLenum, textarget: types::GLenum, texture: types::GLuint, level: types::GLint, zoffset: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLenum, types::GLuint, types::GLint, types::GLint) -> ()>(storage::FramebufferTexture3D.f)(target, attachment, textarget, texture, level, zoffset) }
/// Fallbacks: FramebufferTextureLayerARB, FramebufferTextureLayerEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn FramebufferTextureLayer(target: types::GLenum, attachment: types::GLenum, texture: types::GLuint, level: types::GLint, layer: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLuint, types::GLint, types::GLint) -> ()>(storage::FramebufferTextureLayer.f)(target, attachment, texture, level, layer) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn FrontFace(mode: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> ()>(storage::FrontFace.f)(mode) }
/// Fallbacks: GenBuffersARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GenBuffers(n: types::GLsizei, buffers: *mut types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsizei, *mut types::GLuint) -> ()>(storage::GenBuffers.f)(n, buffers) }
/// Fallbacks: GenFramebuffersEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GenFramebuffers(n: types::GLsizei, framebuffers: *mut types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsizei, *mut types::GLuint) -> ()>(storage::GenFramebuffers.f)(n, framebuffers) }
/// Fallbacks: GenQueriesARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GenQueries(n: types::GLsizei, ids: *mut types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsizei, *mut types::GLuint) -> ()>(storage::GenQueries.f)(n, ids) }
/// Fallbacks: GenRenderbuffersEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GenRenderbuffers(n: types::GLsizei, renderbuffers: *mut types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsizei, *mut types::GLuint) -> ()>(storage::GenRenderbuffers.f)(n, renderbuffers) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GenSamplers(count: types::GLsizei, samplers: *mut types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsizei, *mut types::GLuint) -> ()>(storage::GenSamplers.f)(count, samplers) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GenTextures(n: types::GLsizei, textures: *mut types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsizei, *mut types::GLuint) -> ()>(storage::GenTextures.f)(n, textures) }
/// Fallbacks: GenVertexArraysAPPLE, GenVertexArraysOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GenVertexArrays(n: types::GLsizei, arrays: *mut types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsizei, *mut types::GLuint) -> ()>(storage::GenVertexArrays.f)(n, arrays) }
/// Fallbacks: GenerateMipmapEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GenerateMipmap(target: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> ()>(storage::GenerateMipmap.f)(target) }
/// Fallbacks: GetActiveAttribARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetActiveAttrib(program: types::GLuint, index: types::GLuint, bufSize: types::GLsizei, length: *mut types::GLsizei, size: *mut types::GLint, type_: *mut types::GLenum, name: *mut types::GLchar) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLuint, types::GLsizei, *mut types::GLsizei, *mut types::GLint, *mut types::GLenum, *mut types::GLchar) -> ()>(storage::GetActiveAttrib.f)(program, index, bufSize, length, size, type_, name) }
/// Fallbacks: GetActiveUniformARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetActiveUniform(program: types::GLuint, index: types::GLuint, bufSize: types::GLsizei, length: *mut types::GLsizei, size: *mut types::GLint, type_: *mut types::GLenum, name: *mut types::GLchar) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLuint, types::GLsizei, *mut types::GLsizei, *mut types::GLint, *mut types::GLenum, *mut types::GLchar) -> ()>(storage::GetActiveUniform.f)(program, index, bufSize, length, size, type_, name) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetActiveUniformBlockName(program: types::GLuint, uniformBlockIndex: types::GLuint, bufSize: types::GLsizei, length: *mut types::GLsizei, uniformBlockName: *mut types::GLchar) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLuint, types::GLsizei, *mut types::GLsizei, *mut types::GLchar) -> ()>(storage::GetActiveUniformBlockName.f)(program, uniformBlockIndex, bufSize, length, uniformBlockName) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetActiveUniformBlockiv(program: types::GLuint, uniformBlockIndex: types::GLuint, pname: types::GLenum, params: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLuint, types::GLenum, *mut types::GLint) -> ()>(storage::GetActiveUniformBlockiv.f)(program, uniformBlockIndex, pname, params) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetActiveUniformName(program: types::GLuint, uniformIndex: types::GLuint, bufSize: types::GLsizei, length: *mut types::GLsizei, uniformName: *mut types::GLchar) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLuint, types::GLsizei, *mut types::GLsizei, *mut types::GLchar) -> ()>(storage::GetActiveUniformName.f)(program, uniformIndex, bufSize, length, uniformName) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetActiveUniformsiv(program: types::GLuint, uniformCount: types::GLsizei, uniformIndices: *const types::GLuint, pname: types::GLenum, params: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLsizei, *const types::GLuint, types::GLenum, *mut types::GLint) -> ()>(storage::GetActiveUniformsiv.f)(program, uniformCount, uniformIndices, pname, params) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetAttachedShaders(program: types::GLuint, maxCount: types::GLsizei, count: *mut types::GLsizei, shaders: *mut types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLsizei, *mut types::GLsizei, *mut types::GLuint) -> ()>(storage::GetAttachedShaders.f)(program, maxCount, count, shaders) }
/// Fallbacks: GetAttribLocationARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetAttribLocation(program: types::GLuint, name: *const types::GLchar) -> types::GLint { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLchar) -> types::GLint>(storage::GetAttribLocation.f)(program, name) }
/// Fallbacks: GetBooleanIndexedvEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetBooleani_v(target: types::GLenum, index: types::GLuint, data: *mut types::GLboolean) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint, *mut types::GLboolean) -> ()>(storage::GetBooleani_v.f)(target, index, data) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetBooleanv(pname: types::GLenum, data: *mut types::GLboolean) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *mut types::GLboolean) -> ()>(storage::GetBooleanv.f)(pname, data) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetBufferParameteri64v(target: types::GLenum, pname: types::GLenum, params: *mut types::GLint64) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, *mut types::GLint64) -> ()>(storage::GetBufferParameteri64v.f)(target, pname, params) }
/// Fallbacks: GetBufferParameterivARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetBufferParameteriv(target: types::GLenum, pname: types::GLenum, params: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, *mut types::GLint) -> ()>(storage::GetBufferParameteriv.f)(target, pname, params) }
/// Fallbacks: GetBufferPointervARB, GetBufferPointervOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetBufferPointerv(target: types::GLenum, pname: types::GLenum, params: *const *mut __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, *const *mut __gl_imports::raw::c_void) -> ()>(storage::GetBufferPointerv.f)(target, pname, params) }
/// Fallbacks: GetBufferSubDataARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetBufferSubData(target: types::GLenum, offset: types::GLintptr, size: types::GLsizeiptr, data: *mut __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLintptr, types::GLsizeiptr, *mut __gl_imports::raw::c_void) -> ()>(storage::GetBufferSubData.f)(target, offset, size, data) }
/// Fallbacks: GetCompressedTexImageARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetCompressedTexImage(target: types::GLenum, level: types::GLint, img: *mut __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, *mut __gl_imports::raw::c_void) -> ()>(storage::GetCompressedTexImage.f)(target, level, img) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetDoublev(pname: types::GLenum, data: *mut types::GLdouble) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *mut types::GLdouble) -> ()>(storage::GetDoublev.f)(pname, data) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetError() -> types::GLenum { __gl_imports::mem::transmute::<_, extern "system" fn() -> types::GLenum>(storage::GetError.f)() }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetFloatv(pname: types::GLenum, data: *mut types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *mut types::GLfloat) -> ()>(storage::GetFloatv.f)(pname, data) }
/// Fallbacks: GetFragDataIndexEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetFragDataIndex(program: types::GLuint, name: *const types::GLchar) -> types::GLint { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLchar) -> types::GLint>(storage::GetFragDataIndex.f)(program, name) }
/// Fallbacks: GetFragDataLocationEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetFragDataLocation(program: types::GLuint, name: *const types::GLchar) -> types::GLint { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLchar) -> types::GLint>(storage::GetFragDataLocation.f)(program, name) }
/// Fallbacks: GetFramebufferAttachmentParameterivEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetFramebufferAttachmentParameteriv(target: types::GLenum, attachment: types::GLenum, pname: types::GLenum, params: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLenum, *mut types::GLint) -> ()>(storage::GetFramebufferAttachmentParameteriv.f)(target, attachment, pname, params) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetInteger64i_v(target: types::GLenum, index: types::GLuint, data: *mut types::GLint64) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint, *mut types::GLint64) -> ()>(storage::GetInteger64i_v.f)(target, index, data) }
/// Fallbacks: GetInteger64vAPPLE
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetInteger64v(pname: types::GLenum, data: *mut types::GLint64) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *mut types::GLint64) -> ()>(storage::GetInteger64v.f)(pname, data) }
/// Fallbacks: GetIntegerIndexedvEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetIntegeri_v(target: types::GLenum, index: types::GLuint, data: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint, *mut types::GLint) -> ()>(storage::GetIntegeri_v.f)(target, index, data) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetIntegerv(pname: types::GLenum, data: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *mut types::GLint) -> ()>(storage::GetIntegerv.f)(pname, data) }
/// Fallbacks: GetMultisamplefvNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetMultisamplefv(pname: types::GLenum, index: types::GLuint, val: *mut types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint, *mut types::GLfloat) -> ()>(storage::GetMultisamplefv.f)(pname, index, val) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetProgramInfoLog(program: types::GLuint, bufSize: types::GLsizei, length: *mut types::GLsizei, infoLog: *mut types::GLchar) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLsizei, *mut types::GLsizei, *mut types::GLchar) -> ()>(storage::GetProgramInfoLog.f)(program, bufSize, length, infoLog) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetProgramiv(program: types::GLuint, pname: types::GLenum, params: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *mut types::GLint) -> ()>(storage::GetProgramiv.f)(program, pname, params) }
/// Fallbacks: GetQueryObjecti64vEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetQueryObjecti64v(id: types::GLuint, pname: types::GLenum, params: *mut types::GLint64) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *mut types::GLint64) -> ()>(storage::GetQueryObjecti64v.f)(id, pname, params) }
/// Fallbacks: GetQueryObjectivARB, GetQueryObjectivEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetQueryObjectiv(id: types::GLuint, pname: types::GLenum, params: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *mut types::GLint) -> ()>(storage::GetQueryObjectiv.f)(id, pname, params) }
/// Fallbacks: GetQueryObjectui64vEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetQueryObjectui64v(id: types::GLuint, pname: types::GLenum, params: *mut types::GLuint64) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *mut types::GLuint64) -> ()>(storage::GetQueryObjectui64v.f)(id, pname, params) }
/// Fallbacks: GetQueryObjectuivARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetQueryObjectuiv(id: types::GLuint, pname: types::GLenum, params: *mut types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *mut types::GLuint) -> ()>(storage::GetQueryObjectuiv.f)(id, pname, params) }
/// Fallbacks: GetQueryivARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetQueryiv(target: types::GLenum, pname: types::GLenum, params: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, *mut types::GLint) -> ()>(storage::GetQueryiv.f)(target, pname, params) }
/// Fallbacks: GetRenderbufferParameterivEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetRenderbufferParameteriv(target: types::GLenum, pname: types::GLenum, params: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, *mut types::GLint) -> ()>(storage::GetRenderbufferParameteriv.f)(target, pname, params) }
/// Fallbacks: GetSamplerParameterIivEXT, GetSamplerParameterIivOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetSamplerParameterIiv(sampler: types::GLuint, pname: types::GLenum, params: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *mut types::GLint) -> ()>(storage::GetSamplerParameterIiv.f)(sampler, pname, params) }
/// Fallbacks: GetSamplerParameterIuivEXT, GetSamplerParameterIuivOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetSamplerParameterIuiv(sampler: types::GLuint, pname: types::GLenum, params: *mut types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *mut types::GLuint) -> ()>(storage::GetSamplerParameterIuiv.f)(sampler, pname, params) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetSamplerParameterfv(sampler: types::GLuint, pname: types::GLenum, params: *mut types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *mut types::GLfloat) -> ()>(storage::GetSamplerParameterfv.f)(sampler, pname, params) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetSamplerParameteriv(sampler: types::GLuint, pname: types::GLenum, params: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *mut types::GLint) -> ()>(storage::GetSamplerParameteriv.f)(sampler, pname, params) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetShaderInfoLog(shader: types::GLuint, bufSize: types::GLsizei, length: *mut types::GLsizei, infoLog: *mut types::GLchar) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLsizei, *mut types::GLsizei, *mut types::GLchar) -> ()>(storage::GetShaderInfoLog.f)(shader, bufSize, length, infoLog) }
/// Fallbacks: GetShaderSourceARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetShaderSource(shader: types::GLuint, bufSize: types::GLsizei, length: *mut types::GLsizei, source: *mut types::GLchar) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLsizei, *mut types::GLsizei, *mut types::GLchar) -> ()>(storage::GetShaderSource.f)(shader, bufSize, length, source) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetShaderiv(shader: types::GLuint, pname: types::GLenum, params: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *mut types::GLint) -> ()>(storage::GetShaderiv.f)(shader, pname, params) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetString(name: types::GLenum) -> *const types::GLubyte { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> *const types::GLubyte>(storage::GetString.f)(name) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetStringi(name: types::GLenum, index: types::GLuint) -> *const types::GLubyte { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> *const types::GLubyte>(storage::GetStringi.f)(name, index) }
/// Fallbacks: GetSyncivAPPLE
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetSynciv(sync: types::GLsync, pname: types::GLenum, bufSize: types::GLsizei, length: *mut types::GLsizei, values: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsync, types::GLenum, types::GLsizei, *mut types::GLsizei, *mut types::GLint) -> ()>(storage::GetSynciv.f)(sync, pname, bufSize, length, values) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetTexImage(target: types::GLenum, level: types::GLint, format: types::GLenum, type_: types::GLenum, pixels: *mut __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLenum, types::GLenum, *mut __gl_imports::raw::c_void) -> ()>(storage::GetTexImage.f)(target, level, format, type_, pixels) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetTexLevelParameterfv(target: types::GLenum, level: types::GLint, pname: types::GLenum, params: *mut types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLenum, *mut types::GLfloat) -> ()>(storage::GetTexLevelParameterfv.f)(target, level, pname, params) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetTexLevelParameteriv(target: types::GLenum, level: types::GLint, pname: types::GLenum, params: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLenum, *mut types::GLint) -> ()>(storage::GetTexLevelParameteriv.f)(target, level, pname, params) }
/// Fallbacks: GetTexParameterIivEXT, GetTexParameterIivOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetTexParameterIiv(target: types::GLenum, pname: types::GLenum, params: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, *mut types::GLint) -> ()>(storage::GetTexParameterIiv.f)(target, pname, params) }
/// Fallbacks: GetTexParameterIuivEXT, GetTexParameterIuivOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetTexParameterIuiv(target: types::GLenum, pname: types::GLenum, params: *mut types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, *mut types::GLuint) -> ()>(storage::GetTexParameterIuiv.f)(target, pname, params) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetTexParameterfv(target: types::GLenum, pname: types::GLenum, params: *mut types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, *mut types::GLfloat) -> ()>(storage::GetTexParameterfv.f)(target, pname, params) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetTexParameteriv(target: types::GLenum, pname: types::GLenum, params: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, *mut types::GLint) -> ()>(storage::GetTexParameteriv.f)(target, pname, params) }
/// Fallbacks: GetTransformFeedbackVaryingEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetTransformFeedbackVarying(program: types::GLuint, index: types::GLuint, bufSize: types::GLsizei, length: *mut types::GLsizei, size: *mut types::GLsizei, type_: *mut types::GLenum, name: *mut types::GLchar) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLuint, types::GLsizei, *mut types::GLsizei, *mut types::GLsizei, *mut types::GLenum, *mut types::GLchar) -> ()>(storage::GetTransformFeedbackVarying.f)(program, index, bufSize, length, size, type_, name) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetUniformBlockIndex(program: types::GLuint, uniformBlockName: *const types::GLchar) -> types::GLuint { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLchar) -> types::GLuint>(storage::GetUniformBlockIndex.f)(program, uniformBlockName) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetUniformIndices(program: types::GLuint, uniformCount: types::GLsizei, uniformNames: *const *const types::GLchar, uniformIndices: *mut types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLsizei, *const *const types::GLchar, *mut types::GLuint) -> ()>(storage::GetUniformIndices.f)(program, uniformCount, uniformNames, uniformIndices) }
/// Fallbacks: GetUniformLocationARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetUniformLocation(program: types::GLuint, name: *const types::GLchar) -> types::GLint { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLchar) -> types::GLint>(storage::GetUniformLocation.f)(program, name) }
/// Fallbacks: GetUniformfvARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetUniformfv(program: types::GLuint, location: types::GLint, params: *mut types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLint, *mut types::GLfloat) -> ()>(storage::GetUniformfv.f)(program, location, params) }
/// Fallbacks: GetUniformivARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetUniformiv(program: types::GLuint, location: types::GLint, params: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLint, *mut types::GLint) -> ()>(storage::GetUniformiv.f)(program, location, params) }
/// Fallbacks: GetUniformuivEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetUniformuiv(program: types::GLuint, location: types::GLint, params: *mut types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLint, *mut types::GLuint) -> ()>(storage::GetUniformuiv.f)(program, location, params) }
/// Fallbacks: GetVertexAttribIivEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetVertexAttribIiv(index: types::GLuint, pname: types::GLenum, params: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *mut types::GLint) -> ()>(storage::GetVertexAttribIiv.f)(index, pname, params) }
/// Fallbacks: GetVertexAttribIuivEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetVertexAttribIuiv(index: types::GLuint, pname: types::GLenum, params: *mut types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *mut types::GLuint) -> ()>(storage::GetVertexAttribIuiv.f)(index, pname, params) }
/// Fallbacks: GetVertexAttribPointervARB, GetVertexAttribPointervNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetVertexAttribPointerv(index: types::GLuint, pname: types::GLenum, pointer: *const *mut __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *const *mut __gl_imports::raw::c_void) -> ()>(storage::GetVertexAttribPointerv.f)(index, pname, pointer) }
/// Fallbacks: GetVertexAttribdvARB, GetVertexAttribdvNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetVertexAttribdv(index: types::GLuint, pname: types::GLenum, params: *mut types::GLdouble) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *mut types::GLdouble) -> ()>(storage::GetVertexAttribdv.f)(index, pname, params) }
/// Fallbacks: GetVertexAttribfvARB, GetVertexAttribfvNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetVertexAttribfv(index: types::GLuint, pname: types::GLenum, params: *mut types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *mut types::GLfloat) -> ()>(storage::GetVertexAttribfv.f)(index, pname, params) }
/// Fallbacks: GetVertexAttribivARB, GetVertexAttribivNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn GetVertexAttribiv(index: types::GLuint, pname: types::GLenum, params: *mut types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *mut types::GLint) -> ()>(storage::GetVertexAttribiv.f)(index, pname, params) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Hint(target: types::GLenum, mode: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum) -> ()>(storage::Hint.f)(target, mode) }
/// Fallbacks: IsBufferARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn IsBuffer(buffer: types::GLuint) -> types::GLboolean { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> types::GLboolean>(storage::IsBuffer.f)(buffer) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn IsEnabled(cap: types::GLenum) -> types::GLboolean { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> types::GLboolean>(storage::IsEnabled.f)(cap) }
/// Fallbacks: IsEnabledIndexedEXT, IsEnablediEXT, IsEnablediNV, IsEnablediOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn IsEnabledi(target: types::GLenum, index: types::GLuint) -> types::GLboolean { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> types::GLboolean>(storage::IsEnabledi.f)(target, index) }
/// Fallbacks: IsFramebufferEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn IsFramebuffer(framebuffer: types::GLuint) -> types::GLboolean { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> types::GLboolean>(storage::IsFramebuffer.f)(framebuffer) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn IsProgram(program: types::GLuint) -> types::GLboolean { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> types::GLboolean>(storage::IsProgram.f)(program) }
/// Fallbacks: IsQueryARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn IsQuery(id: types::GLuint) -> types::GLboolean { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> types::GLboolean>(storage::IsQuery.f)(id) }
/// Fallbacks: IsRenderbufferEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn IsRenderbuffer(renderbuffer: types::GLuint) -> types::GLboolean { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> types::GLboolean>(storage::IsRenderbuffer.f)(renderbuffer) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn IsSampler(sampler: types::GLuint) -> types::GLboolean { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> types::GLboolean>(storage::IsSampler.f)(sampler) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn IsShader(shader: types::GLuint) -> types::GLboolean { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> types::GLboolean>(storage::IsShader.f)(shader) }
/// Fallbacks: IsSyncAPPLE
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn IsSync(sync: types::GLsync) -> types::GLboolean { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsync) -> types::GLboolean>(storage::IsSync.f)(sync) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn IsTexture(texture: types::GLuint) -> types::GLboolean { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> types::GLboolean>(storage::IsTexture.f)(texture) }
/// Fallbacks: IsVertexArrayAPPLE, IsVertexArrayOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn IsVertexArray(array: types::GLuint) -> types::GLboolean { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> types::GLboolean>(storage::IsVertexArray.f)(array) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn LineWidth(width: types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLfloat) -> ()>(storage::LineWidth.f)(width) }
/// Fallbacks: LinkProgramARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn LinkProgram(program: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> ()>(storage::LinkProgram.f)(program) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn LogicOp(opcode: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> ()>(storage::LogicOp.f)(opcode) }
/// Fallbacks: MapBufferARB, MapBufferOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn MapBuffer(target: types::GLenum, access: types::GLenum) -> *mut __gl_imports::raw::c_void { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum) -> *mut __gl_imports::raw::c_void>(storage::MapBuffer.f)(target, access) }
/// Fallbacks: MapBufferRangeEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn MapBufferRange(target: types::GLenum, offset: types::GLintptr, length: types::GLsizeiptr, access: types::GLbitfield) -> *mut __gl_imports::raw::c_void { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLintptr, types::GLsizeiptr, types::GLbitfield) -> *mut __gl_imports::raw::c_void>(storage::MapBufferRange.f)(target, offset, length, access) }
/// Fallbacks: MultiDrawArraysEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn MultiDrawArrays(mode: types::GLenum, first: *const types::GLint, count: *const types::GLsizei, drawcount: types::GLsizei) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *const types::GLint, *const types::GLsizei, types::GLsizei) -> ()>(storage::MultiDrawArrays.f)(mode, first, count, drawcount) }
/// Fallbacks: MultiDrawElementsEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn MultiDrawElements(mode: types::GLenum, count: *const types::GLsizei, type_: types::GLenum, indices: *const *const __gl_imports::raw::c_void, drawcount: types::GLsizei) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *const types::GLsizei, types::GLenum, *const *const __gl_imports::raw::c_void, types::GLsizei) -> ()>(storage::MultiDrawElements.f)(mode, count, type_, indices, drawcount) }
/// Fallbacks: MultiDrawElementsBaseVertexEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn MultiDrawElementsBaseVertex(mode: types::GLenum, count: *const types::GLsizei, type_: types::GLenum, indices: *const *const __gl_imports::raw::c_void, drawcount: types::GLsizei, basevertex: *const types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *const types::GLsizei, types::GLenum, *const *const __gl_imports::raw::c_void, types::GLsizei, *const types::GLint) -> ()>(storage::MultiDrawElementsBaseVertex.f)(mode, count, type_, indices, drawcount, basevertex) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn MultiTexCoordP1ui(texture: types::GLenum, type_: types::GLenum, coords: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLuint) -> ()>(storage::MultiTexCoordP1ui.f)(texture, type_, coords) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn MultiTexCoordP1uiv(texture: types::GLenum, type_: types::GLenum, coords: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, *const types::GLuint) -> ()>(storage::MultiTexCoordP1uiv.f)(texture, type_, coords) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn MultiTexCoordP2ui(texture: types::GLenum, type_: types::GLenum, coords: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLuint) -> ()>(storage::MultiTexCoordP2ui.f)(texture, type_, coords) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn MultiTexCoordP2uiv(texture: types::GLenum, type_: types::GLenum, coords: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, *const types::GLuint) -> ()>(storage::MultiTexCoordP2uiv.f)(texture, type_, coords) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn MultiTexCoordP3ui(texture: types::GLenum, type_: types::GLenum, coords: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLuint) -> ()>(storage::MultiTexCoordP3ui.f)(texture, type_, coords) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn MultiTexCoordP3uiv(texture: types::GLenum, type_: types::GLenum, coords: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, *const types::GLuint) -> ()>(storage::MultiTexCoordP3uiv.f)(texture, type_, coords) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn MultiTexCoordP4ui(texture: types::GLenum, type_: types::GLenum, coords: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLuint) -> ()>(storage::MultiTexCoordP4ui.f)(texture, type_, coords) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn MultiTexCoordP4uiv(texture: types::GLenum, type_: types::GLenum, coords: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, *const types::GLuint) -> ()>(storage::MultiTexCoordP4uiv.f)(texture, type_, coords) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn NormalP3ui(type_: types::GLenum, coords: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::NormalP3ui.f)(type_, coords) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn NormalP3uiv(type_: types::GLenum, coords: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *const types::GLuint) -> ()>(storage::NormalP3uiv.f)(type_, coords) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn PixelStoref(pname: types::GLenum, param: types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLfloat) -> ()>(storage::PixelStoref.f)(pname, param) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn PixelStorei(pname: types::GLenum, param: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint) -> ()>(storage::PixelStorei.f)(pname, param) }
/// Fallbacks: PointParameterfARB, PointParameterfEXT, PointParameterfSGIS
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn PointParameterf(pname: types::GLenum, param: types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLfloat) -> ()>(storage::PointParameterf.f)(pname, param) }
/// Fallbacks: PointParameterfvARB, PointParameterfvEXT, PointParameterfvSGIS
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn PointParameterfv(pname: types::GLenum, params: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *const types::GLfloat) -> ()>(storage::PointParameterfv.f)(pname, params) }
/// Fallbacks: PointParameteriNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn PointParameteri(pname: types::GLenum, param: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint) -> ()>(storage::PointParameteri.f)(pname, param) }
/// Fallbacks: PointParameterivNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn PointParameteriv(pname: types::GLenum, params: *const types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *const types::GLint) -> ()>(storage::PointParameteriv.f)(pname, params) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn PointSize(size: types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLfloat) -> ()>(storage::PointSize.f)(size) }
/// Fallbacks: PolygonModeNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn PolygonMode(face: types::GLenum, mode: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum) -> ()>(storage::PolygonMode.f)(face, mode) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn PolygonOffset(factor: types::GLfloat, units: types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLfloat, types::GLfloat) -> ()>(storage::PolygonOffset.f)(factor, units) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn PrimitiveRestartIndex(index: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> ()>(storage::PrimitiveRestartIndex.f)(index) }
/// Fallbacks: ProvokingVertexEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ProvokingVertex(mode: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> ()>(storage::ProvokingVertex.f)(mode) }
/// Fallbacks: QueryCounterEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn QueryCounter(id: types::GLuint, target: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum) -> ()>(storage::QueryCounter.f)(id, target) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ReadBuffer(src: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> ()>(storage::ReadBuffer.f)(src) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ReadPixels(x: types::GLint, y: types::GLint, width: types::GLsizei, height: types::GLsizei, format: types::GLenum, type_: types::GLenum, pixels: *mut __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLint, types::GLsizei, types::GLsizei, types::GLenum, types::GLenum, *mut __gl_imports::raw::c_void) -> ()>(storage::ReadPixels.f)(x, y, width, height, format, type_, pixels) }
/// Fallbacks: RenderbufferStorageEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn RenderbufferStorage(target: types::GLenum, internalformat: types::GLenum, width: types::GLsizei, height: types::GLsizei) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLsizei, types::GLsizei) -> ()>(storage::RenderbufferStorage.f)(target, internalformat, width, height) }
/// Fallbacks: RenderbufferStorageMultisampleEXT, RenderbufferStorageMultisampleNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn RenderbufferStorageMultisample(target: types::GLenum, samples: types::GLsizei, internalformat: types::GLenum, width: types::GLsizei, height: types::GLsizei) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLsizei, types::GLenum, types::GLsizei, types::GLsizei) -> ()>(storage::RenderbufferStorageMultisample.f)(target, samples, internalformat, width, height) }
/// Fallbacks: SampleCoverageARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn SampleCoverage(value: types::GLfloat, invert: types::GLboolean) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLfloat, types::GLboolean) -> ()>(storage::SampleCoverage.f)(value, invert) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn SampleMaski(maskNumber: types::GLuint, mask: types::GLbitfield) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLbitfield) -> ()>(storage::SampleMaski.f)(maskNumber, mask) }
/// Fallbacks: SamplerParameterIivEXT, SamplerParameterIivOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn SamplerParameterIiv(sampler: types::GLuint, pname: types::GLenum, param: *const types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *const types::GLint) -> ()>(storage::SamplerParameterIiv.f)(sampler, pname, param) }
/// Fallbacks: SamplerParameterIuivEXT, SamplerParameterIuivOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn SamplerParameterIuiv(sampler: types::GLuint, pname: types::GLenum, param: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *const types::GLuint) -> ()>(storage::SamplerParameterIuiv.f)(sampler, pname, param) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn SamplerParameterf(sampler: types::GLuint, pname: types::GLenum, param: types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, types::GLfloat) -> ()>(storage::SamplerParameterf.f)(sampler, pname, param) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn SamplerParameterfv(sampler: types::GLuint, pname: types::GLenum, param: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *const types::GLfloat) -> ()>(storage::SamplerParameterfv.f)(sampler, pname, param) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn SamplerParameteri(sampler: types::GLuint, pname: types::GLenum, param: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, types::GLint) -> ()>(storage::SamplerParameteri.f)(sampler, pname, param) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn SamplerParameteriv(sampler: types::GLuint, pname: types::GLenum, param: *const types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, *const types::GLint) -> ()>(storage::SamplerParameteriv.f)(sampler, pname, param) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Scissor(x: types::GLint, y: types::GLint, width: types::GLsizei, height: types::GLsizei) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLint, types::GLsizei, types::GLsizei) -> ()>(storage::Scissor.f)(x, y, width, height) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn SecondaryColorP3ui(type_: types::GLenum, color: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::SecondaryColorP3ui.f)(type_, color) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn SecondaryColorP3uiv(type_: types::GLenum, color: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *const types::GLuint) -> ()>(storage::SecondaryColorP3uiv.f)(type_, color) }
/// Fallbacks: ShaderSourceARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ShaderSource(shader: types::GLuint, count: types::GLsizei, string: *const *const types::GLchar, length: *const types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLsizei, *const *const types::GLchar, *const types::GLint) -> ()>(storage::ShaderSource.f)(shader, count, string, length) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn StencilFunc(func: types::GLenum, ref_: types::GLint, mask: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLuint) -> ()>(storage::StencilFunc.f)(func, ref_, mask) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn StencilFuncSeparate(face: types::GLenum, func: types::GLenum, ref_: types::GLint, mask: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLint, types::GLuint) -> ()>(storage::StencilFuncSeparate.f)(face, func, ref_, mask) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn StencilMask(mask: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> ()>(storage::StencilMask.f)(mask) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn StencilMaskSeparate(face: types::GLenum, mask: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::StencilMaskSeparate.f)(face, mask) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn StencilOp(fail: types::GLenum, zfail: types::GLenum, zpass: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLenum) -> ()>(storage::StencilOp.f)(fail, zfail, zpass) }
/// Fallbacks: StencilOpSeparateATI
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn StencilOpSeparate(face: types::GLenum, sfail: types::GLenum, dpfail: types::GLenum, dppass: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLenum, types::GLenum) -> ()>(storage::StencilOpSeparate.f)(face, sfail, dpfail, dppass) }
/// Fallbacks: TexBufferARB, TexBufferEXT, TexBufferOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexBuffer(target: types::GLenum, internalformat: types::GLenum, buffer: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLuint) -> ()>(storage::TexBuffer.f)(target, internalformat, buffer) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexCoordP1ui(type_: types::GLenum, coords: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::TexCoordP1ui.f)(type_, coords) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexCoordP1uiv(type_: types::GLenum, coords: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *const types::GLuint) -> ()>(storage::TexCoordP1uiv.f)(type_, coords) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexCoordP2ui(type_: types::GLenum, coords: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::TexCoordP2ui.f)(type_, coords) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexCoordP2uiv(type_: types::GLenum, coords: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *const types::GLuint) -> ()>(storage::TexCoordP2uiv.f)(type_, coords) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexCoordP3ui(type_: types::GLenum, coords: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::TexCoordP3ui.f)(type_, coords) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexCoordP3uiv(type_: types::GLenum, coords: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *const types::GLuint) -> ()>(storage::TexCoordP3uiv.f)(type_, coords) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexCoordP4ui(type_: types::GLenum, coords: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::TexCoordP4ui.f)(type_, coords) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexCoordP4uiv(type_: types::GLenum, coords: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *const types::GLuint) -> ()>(storage::TexCoordP4uiv.f)(type_, coords) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexImage1D(target: types::GLenum, level: types::GLint, internalformat: types::GLint, width: types::GLsizei, border: types::GLint, format: types::GLenum, type_: types::GLenum, pixels: *const __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLint, types::GLsizei, types::GLint, types::GLenum, types::GLenum, *const __gl_imports::raw::c_void) -> ()>(storage::TexImage1D.f)(target, level, internalformat, width, border, format, type_, pixels) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexImage2D(target: types::GLenum, level: types::GLint, internalformat: types::GLint, width: types::GLsizei, height: types::GLsizei, border: types::GLint, format: types::GLenum, type_: types::GLenum, pixels: *const __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLint, types::GLsizei, types::GLsizei, types::GLint, types::GLenum, types::GLenum, *const __gl_imports::raw::c_void) -> ()>(storage::TexImage2D.f)(target, level, internalformat, width, height, border, format, type_, pixels) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexImage2DMultisample(target: types::GLenum, samples: types::GLsizei, internalformat: types::GLenum, width: types::GLsizei, height: types::GLsizei, fixedsamplelocations: types::GLboolean) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLsizei, types::GLenum, types::GLsizei, types::GLsizei, types::GLboolean) -> ()>(storage::TexImage2DMultisample.f)(target, samples, internalformat, width, height, fixedsamplelocations) }
/// Fallbacks: TexImage3DEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexImage3D(target: types::GLenum, level: types::GLint, internalformat: types::GLint, width: types::GLsizei, height: types::GLsizei, depth: types::GLsizei, border: types::GLint, format: types::GLenum, type_: types::GLenum, pixels: *const __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLint, types::GLsizei, types::GLsizei, types::GLsizei, types::GLint, types::GLenum, types::GLenum, *const __gl_imports::raw::c_void) -> ()>(storage::TexImage3D.f)(target, level, internalformat, width, height, depth, border, format, type_, pixels) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexImage3DMultisample(target: types::GLenum, samples: types::GLsizei, internalformat: types::GLenum, width: types::GLsizei, height: types::GLsizei, depth: types::GLsizei, fixedsamplelocations: types::GLboolean) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLsizei, types::GLenum, types::GLsizei, types::GLsizei, types::GLsizei, types::GLboolean) -> ()>(storage::TexImage3DMultisample.f)(target, samples, internalformat, width, height, depth, fixedsamplelocations) }
/// Fallbacks: TexParameterIivEXT, TexParameterIivOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexParameterIiv(target: types::GLenum, pname: types::GLenum, params: *const types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, *const types::GLint) -> ()>(storage::TexParameterIiv.f)(target, pname, params) }
/// Fallbacks: TexParameterIuivEXT, TexParameterIuivOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexParameterIuiv(target: types::GLenum, pname: types::GLenum, params: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, *const types::GLuint) -> ()>(storage::TexParameterIuiv.f)(target, pname, params) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexParameterf(target: types::GLenum, pname: types::GLenum, param: types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLfloat) -> ()>(storage::TexParameterf.f)(target, pname, param) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexParameterfv(target: types::GLenum, pname: types::GLenum, params: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, *const types::GLfloat) -> ()>(storage::TexParameterfv.f)(target, pname, params) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexParameteri(target: types::GLenum, pname: types::GLenum, param: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, types::GLint) -> ()>(storage::TexParameteri.f)(target, pname, param) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexParameteriv(target: types::GLenum, pname: types::GLenum, params: *const types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLenum, *const types::GLint) -> ()>(storage::TexParameteriv.f)(target, pname, params) }
/// Fallbacks: TexSubImage1DEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexSubImage1D(target: types::GLenum, level: types::GLint, xoffset: types::GLint, width: types::GLsizei, format: types::GLenum, type_: types::GLenum, pixels: *const __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLint, types::GLsizei, types::GLenum, types::GLenum, *const __gl_imports::raw::c_void) -> ()>(storage::TexSubImage1D.f)(target, level, xoffset, width, format, type_, pixels) }
/// Fallbacks: TexSubImage2DEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexSubImage2D(target: types::GLenum, level: types::GLint, xoffset: types::GLint, yoffset: types::GLint, width: types::GLsizei, height: types::GLsizei, format: types::GLenum, type_: types::GLenum, pixels: *const __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLint, types::GLint, types::GLsizei, types::GLsizei, types::GLenum, types::GLenum, *const __gl_imports::raw::c_void) -> ()>(storage::TexSubImage2D.f)(target, level, xoffset, yoffset, width, height, format, type_, pixels) }
/// Fallbacks: TexSubImage3DEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TexSubImage3D(target: types::GLenum, level: types::GLint, xoffset: types::GLint, yoffset: types::GLint, zoffset: types::GLint, width: types::GLsizei, height: types::GLsizei, depth: types::GLsizei, format: types::GLenum, type_: types::GLenum, pixels: *const __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLint, types::GLint, types::GLint, types::GLint, types::GLsizei, types::GLsizei, types::GLsizei, types::GLenum, types::GLenum, *const __gl_imports::raw::c_void) -> ()>(storage::TexSubImage3D.f)(target, level, xoffset, yoffset, zoffset, width, height, depth, format, type_, pixels) }
/// Fallbacks: TransformFeedbackVaryingsEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn TransformFeedbackVaryings(program: types::GLuint, count: types::GLsizei, varyings: *const *const types::GLchar, bufferMode: types::GLenum) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLsizei, *const *const types::GLchar, types::GLenum) -> ()>(storage::TransformFeedbackVaryings.f)(program, count, varyings, bufferMode) }
/// Fallbacks: Uniform1fARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform1f(location: types::GLint, v0: types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLfloat) -> ()>(storage::Uniform1f.f)(location, v0) }
/// Fallbacks: Uniform1fvARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform1fv(location: types::GLint, count: types::GLsizei, value: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, *const types::GLfloat) -> ()>(storage::Uniform1fv.f)(location, count, value) }
/// Fallbacks: Uniform1iARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform1i(location: types::GLint, v0: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLint) -> ()>(storage::Uniform1i.f)(location, v0) }
/// Fallbacks: Uniform1ivARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform1iv(location: types::GLint, count: types::GLsizei, value: *const types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, *const types::GLint) -> ()>(storage::Uniform1iv.f)(location, count, value) }
/// Fallbacks: Uniform1uiEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform1ui(location: types::GLint, v0: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLuint) -> ()>(storage::Uniform1ui.f)(location, v0) }
/// Fallbacks: Uniform1uivEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform1uiv(location: types::GLint, count: types::GLsizei, value: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, *const types::GLuint) -> ()>(storage::Uniform1uiv.f)(location, count, value) }
/// Fallbacks: Uniform2fARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform2f(location: types::GLint, v0: types::GLfloat, v1: types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLfloat, types::GLfloat) -> ()>(storage::Uniform2f.f)(location, v0, v1) }
/// Fallbacks: Uniform2fvARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform2fv(location: types::GLint, count: types::GLsizei, value: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, *const types::GLfloat) -> ()>(storage::Uniform2fv.f)(location, count, value) }
/// Fallbacks: Uniform2iARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform2i(location: types::GLint, v0: types::GLint, v1: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLint, types::GLint) -> ()>(storage::Uniform2i.f)(location, v0, v1) }
/// Fallbacks: Uniform2ivARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform2iv(location: types::GLint, count: types::GLsizei, value: *const types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, *const types::GLint) -> ()>(storage::Uniform2iv.f)(location, count, value) }
/// Fallbacks: Uniform2uiEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform2ui(location: types::GLint, v0: types::GLuint, v1: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLuint, types::GLuint) -> ()>(storage::Uniform2ui.f)(location, v0, v1) }
/// Fallbacks: Uniform2uivEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform2uiv(location: types::GLint, count: types::GLsizei, value: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, *const types::GLuint) -> ()>(storage::Uniform2uiv.f)(location, count, value) }
/// Fallbacks: Uniform3fARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform3f(location: types::GLint, v0: types::GLfloat, v1: types::GLfloat, v2: types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLfloat, types::GLfloat, types::GLfloat) -> ()>(storage::Uniform3f.f)(location, v0, v1, v2) }
/// Fallbacks: Uniform3fvARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform3fv(location: types::GLint, count: types::GLsizei, value: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, *const types::GLfloat) -> ()>(storage::Uniform3fv.f)(location, count, value) }
/// Fallbacks: Uniform3iARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform3i(location: types::GLint, v0: types::GLint, v1: types::GLint, v2: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLint, types::GLint, types::GLint) -> ()>(storage::Uniform3i.f)(location, v0, v1, v2) }
/// Fallbacks: Uniform3ivARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform3iv(location: types::GLint, count: types::GLsizei, value: *const types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, *const types::GLint) -> ()>(storage::Uniform3iv.f)(location, count, value) }
/// Fallbacks: Uniform3uiEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform3ui(location: types::GLint, v0: types::GLuint, v1: types::GLuint, v2: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLuint, types::GLuint, types::GLuint) -> ()>(storage::Uniform3ui.f)(location, v0, v1, v2) }
/// Fallbacks: Uniform3uivEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform3uiv(location: types::GLint, count: types::GLsizei, value: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, *const types::GLuint) -> ()>(storage::Uniform3uiv.f)(location, count, value) }
/// Fallbacks: Uniform4fARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform4f(location: types::GLint, v0: types::GLfloat, v1: types::GLfloat, v2: types::GLfloat, v3: types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLfloat, types::GLfloat, types::GLfloat, types::GLfloat) -> ()>(storage::Uniform4f.f)(location, v0, v1, v2, v3) }
/// Fallbacks: Uniform4fvARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform4fv(location: types::GLint, count: types::GLsizei, value: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, *const types::GLfloat) -> ()>(storage::Uniform4fv.f)(location, count, value) }
/// Fallbacks: Uniform4iARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform4i(location: types::GLint, v0: types::GLint, v1: types::GLint, v2: types::GLint, v3: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLint, types::GLint, types::GLint, types::GLint) -> ()>(storage::Uniform4i.f)(location, v0, v1, v2, v3) }
/// Fallbacks: Uniform4ivARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform4iv(location: types::GLint, count: types::GLsizei, value: *const types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, *const types::GLint) -> ()>(storage::Uniform4iv.f)(location, count, value) }
/// Fallbacks: Uniform4uiEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform4ui(location: types::GLint, v0: types::GLuint, v1: types::GLuint, v2: types::GLuint, v3: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLuint, types::GLuint, types::GLuint, types::GLuint) -> ()>(storage::Uniform4ui.f)(location, v0, v1, v2, v3) }
/// Fallbacks: Uniform4uivEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Uniform4uiv(location: types::GLint, count: types::GLsizei, value: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, *const types::GLuint) -> ()>(storage::Uniform4uiv.f)(location, count, value) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn UniformBlockBinding(program: types::GLuint, uniformBlockIndex: types::GLuint, uniformBlockBinding: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLuint, types::GLuint) -> ()>(storage::UniformBlockBinding.f)(program, uniformBlockIndex, uniformBlockBinding) }
/// Fallbacks: UniformMatrix2fvARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn UniformMatrix2fv(location: types::GLint, count: types::GLsizei, transpose: types::GLboolean, value: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, types::GLboolean, *const types::GLfloat) -> ()>(storage::UniformMatrix2fv.f)(location, count, transpose, value) }
/// Fallbacks: UniformMatrix2x3fvNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn UniformMatrix2x3fv(location: types::GLint, count: types::GLsizei, transpose: types::GLboolean, value: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, types::GLboolean, *const types::GLfloat) -> ()>(storage::UniformMatrix2x3fv.f)(location, count, transpose, value) }
/// Fallbacks: UniformMatrix2x4fvNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn UniformMatrix2x4fv(location: types::GLint, count: types::GLsizei, transpose: types::GLboolean, value: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, types::GLboolean, *const types::GLfloat) -> ()>(storage::UniformMatrix2x4fv.f)(location, count, transpose, value) }
/// Fallbacks: UniformMatrix3fvARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn UniformMatrix3fv(location: types::GLint, count: types::GLsizei, transpose: types::GLboolean, value: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, types::GLboolean, *const types::GLfloat) -> ()>(storage::UniformMatrix3fv.f)(location, count, transpose, value) }
/// Fallbacks: UniformMatrix3x2fvNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn UniformMatrix3x2fv(location: types::GLint, count: types::GLsizei, transpose: types::GLboolean, value: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, types::GLboolean, *const types::GLfloat) -> ()>(storage::UniformMatrix3x2fv.f)(location, count, transpose, value) }
/// Fallbacks: UniformMatrix3x4fvNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn UniformMatrix3x4fv(location: types::GLint, count: types::GLsizei, transpose: types::GLboolean, value: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, types::GLboolean, *const types::GLfloat) -> ()>(storage::UniformMatrix3x4fv.f)(location, count, transpose, value) }
/// Fallbacks: UniformMatrix4fvARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn UniformMatrix4fv(location: types::GLint, count: types::GLsizei, transpose: types::GLboolean, value: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, types::GLboolean, *const types::GLfloat) -> ()>(storage::UniformMatrix4fv.f)(location, count, transpose, value) }
/// Fallbacks: UniformMatrix4x2fvNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn UniformMatrix4x2fv(location: types::GLint, count: types::GLsizei, transpose: types::GLboolean, value: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, types::GLboolean, *const types::GLfloat) -> ()>(storage::UniformMatrix4x2fv.f)(location, count, transpose, value) }
/// Fallbacks: UniformMatrix4x3fvNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn UniformMatrix4x3fv(location: types::GLint, count: types::GLsizei, transpose: types::GLboolean, value: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLsizei, types::GLboolean, *const types::GLfloat) -> ()>(storage::UniformMatrix4x3fv.f)(location, count, transpose, value) }
/// Fallbacks: UnmapBufferARB, UnmapBufferOES
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn UnmapBuffer(target: types::GLenum) -> types::GLboolean { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum) -> types::GLboolean>(storage::UnmapBuffer.f)(target) }
/// Fallbacks: UseProgramObjectARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn UseProgram(program: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> ()>(storage::UseProgram.f)(program) }
/// Fallbacks: ValidateProgramARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn ValidateProgram(program: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint) -> ()>(storage::ValidateProgram.f)(program) }
/// Fallbacks: VertexAttrib1dARB, VertexAttrib1dNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib1d(index: types::GLuint, x: types::GLdouble) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLdouble) -> ()>(storage::VertexAttrib1d.f)(index, x) }
/// Fallbacks: VertexAttrib1dvARB, VertexAttrib1dvNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib1dv(index: types::GLuint, v: *const types::GLdouble) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLdouble) -> ()>(storage::VertexAttrib1dv.f)(index, v) }
/// Fallbacks: VertexAttrib1fARB, VertexAttrib1fNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib1f(index: types::GLuint, x: types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLfloat) -> ()>(storage::VertexAttrib1f.f)(index, x) }
/// Fallbacks: VertexAttrib1fvARB, VertexAttrib1fvNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib1fv(index: types::GLuint, v: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLfloat) -> ()>(storage::VertexAttrib1fv.f)(index, v) }
/// Fallbacks: VertexAttrib1sARB, VertexAttrib1sNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib1s(index: types::GLuint, x: types::GLshort) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLshort) -> ()>(storage::VertexAttrib1s.f)(index, x) }
/// Fallbacks: VertexAttrib1svARB, VertexAttrib1svNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib1sv(index: types::GLuint, v: *const types::GLshort) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLshort) -> ()>(storage::VertexAttrib1sv.f)(index, v) }
/// Fallbacks: VertexAttrib2dARB, VertexAttrib2dNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib2d(index: types::GLuint, x: types::GLdouble, y: types::GLdouble) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLdouble, types::GLdouble) -> ()>(storage::VertexAttrib2d.f)(index, x, y) }
/// Fallbacks: VertexAttrib2dvARB, VertexAttrib2dvNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib2dv(index: types::GLuint, v: *const types::GLdouble) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLdouble) -> ()>(storage::VertexAttrib2dv.f)(index, v) }
/// Fallbacks: VertexAttrib2fARB, VertexAttrib2fNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib2f(index: types::GLuint, x: types::GLfloat, y: types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLfloat, types::GLfloat) -> ()>(storage::VertexAttrib2f.f)(index, x, y) }
/// Fallbacks: VertexAttrib2fvARB, VertexAttrib2fvNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib2fv(index: types::GLuint, v: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLfloat) -> ()>(storage::VertexAttrib2fv.f)(index, v) }
/// Fallbacks: VertexAttrib2sARB, VertexAttrib2sNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib2s(index: types::GLuint, x: types::GLshort, y: types::GLshort) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLshort, types::GLshort) -> ()>(storage::VertexAttrib2s.f)(index, x, y) }
/// Fallbacks: VertexAttrib2svARB, VertexAttrib2svNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib2sv(index: types::GLuint, v: *const types::GLshort) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLshort) -> ()>(storage::VertexAttrib2sv.f)(index, v) }
/// Fallbacks: VertexAttrib3dARB, VertexAttrib3dNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib3d(index: types::GLuint, x: types::GLdouble, y: types::GLdouble, z: types::GLdouble) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLdouble, types::GLdouble, types::GLdouble) -> ()>(storage::VertexAttrib3d.f)(index, x, y, z) }
/// Fallbacks: VertexAttrib3dvARB, VertexAttrib3dvNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib3dv(index: types::GLuint, v: *const types::GLdouble) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLdouble) -> ()>(storage::VertexAttrib3dv.f)(index, v) }
/// Fallbacks: VertexAttrib3fARB, VertexAttrib3fNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib3f(index: types::GLuint, x: types::GLfloat, y: types::GLfloat, z: types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLfloat, types::GLfloat, types::GLfloat) -> ()>(storage::VertexAttrib3f.f)(index, x, y, z) }
/// Fallbacks: VertexAttrib3fvARB, VertexAttrib3fvNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib3fv(index: types::GLuint, v: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLfloat) -> ()>(storage::VertexAttrib3fv.f)(index, v) }
/// Fallbacks: VertexAttrib3sARB, VertexAttrib3sNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib3s(index: types::GLuint, x: types::GLshort, y: types::GLshort, z: types::GLshort) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLshort, types::GLshort, types::GLshort) -> ()>(storage::VertexAttrib3s.f)(index, x, y, z) }
/// Fallbacks: VertexAttrib3svARB, VertexAttrib3svNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib3sv(index: types::GLuint, v: *const types::GLshort) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLshort) -> ()>(storage::VertexAttrib3sv.f)(index, v) }
/// Fallbacks: VertexAttrib4NbvARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib4Nbv(index: types::GLuint, v: *const types::GLbyte) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLbyte) -> ()>(storage::VertexAttrib4Nbv.f)(index, v) }
/// Fallbacks: VertexAttrib4NivARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib4Niv(index: types::GLuint, v: *const types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLint) -> ()>(storage::VertexAttrib4Niv.f)(index, v) }
/// Fallbacks: VertexAttrib4NsvARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib4Nsv(index: types::GLuint, v: *const types::GLshort) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLshort) -> ()>(storage::VertexAttrib4Nsv.f)(index, v) }
/// Fallbacks: VertexAttrib4NubARB, VertexAttrib4ubNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib4Nub(index: types::GLuint, x: types::GLubyte, y: types::GLubyte, z: types::GLubyte, w: types::GLubyte) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLubyte, types::GLubyte, types::GLubyte, types::GLubyte) -> ()>(storage::VertexAttrib4Nub.f)(index, x, y, z, w) }
/// Fallbacks: VertexAttrib4NubvARB, VertexAttrib4ubvNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib4Nubv(index: types::GLuint, v: *const types::GLubyte) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLubyte) -> ()>(storage::VertexAttrib4Nubv.f)(index, v) }
/// Fallbacks: VertexAttrib4NuivARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib4Nuiv(index: types::GLuint, v: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLuint) -> ()>(storage::VertexAttrib4Nuiv.f)(index, v) }
/// Fallbacks: VertexAttrib4NusvARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib4Nusv(index: types::GLuint, v: *const types::GLushort) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLushort) -> ()>(storage::VertexAttrib4Nusv.f)(index, v) }
/// Fallbacks: VertexAttrib4bvARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib4bv(index: types::GLuint, v: *const types::GLbyte) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLbyte) -> ()>(storage::VertexAttrib4bv.f)(index, v) }
/// Fallbacks: VertexAttrib4dARB, VertexAttrib4dNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib4d(index: types::GLuint, x: types::GLdouble, y: types::GLdouble, z: types::GLdouble, w: types::GLdouble) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLdouble, types::GLdouble, types::GLdouble, types::GLdouble) -> ()>(storage::VertexAttrib4d.f)(index, x, y, z, w) }
/// Fallbacks: VertexAttrib4dvARB, VertexAttrib4dvNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib4dv(index: types::GLuint, v: *const types::GLdouble) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLdouble) -> ()>(storage::VertexAttrib4dv.f)(index, v) }
/// Fallbacks: VertexAttrib4fARB, VertexAttrib4fNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib4f(index: types::GLuint, x: types::GLfloat, y: types::GLfloat, z: types::GLfloat, w: types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLfloat, types::GLfloat, types::GLfloat, types::GLfloat) -> ()>(storage::VertexAttrib4f.f)(index, x, y, z, w) }
/// Fallbacks: VertexAttrib4fvARB, VertexAttrib4fvNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib4fv(index: types::GLuint, v: *const types::GLfloat) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLfloat) -> ()>(storage::VertexAttrib4fv.f)(index, v) }
/// Fallbacks: VertexAttrib4ivARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib4iv(index: types::GLuint, v: *const types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLint) -> ()>(storage::VertexAttrib4iv.f)(index, v) }
/// Fallbacks: VertexAttrib4sARB, VertexAttrib4sNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib4s(index: types::GLuint, x: types::GLshort, y: types::GLshort, z: types::GLshort, w: types::GLshort) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLshort, types::GLshort, types::GLshort, types::GLshort) -> ()>(storage::VertexAttrib4s.f)(index, x, y, z, w) }
/// Fallbacks: VertexAttrib4svARB, VertexAttrib4svNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib4sv(index: types::GLuint, v: *const types::GLshort) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLshort) -> ()>(storage::VertexAttrib4sv.f)(index, v) }
/// Fallbacks: VertexAttrib4ubvARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib4ubv(index: types::GLuint, v: *const types::GLubyte) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLubyte) -> ()>(storage::VertexAttrib4ubv.f)(index, v) }
/// Fallbacks: VertexAttrib4uivARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib4uiv(index: types::GLuint, v: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLuint) -> ()>(storage::VertexAttrib4uiv.f)(index, v) }
/// Fallbacks: VertexAttrib4usvARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttrib4usv(index: types::GLuint, v: *const types::GLushort) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLushort) -> ()>(storage::VertexAttrib4usv.f)(index, v) }
/// Fallbacks: VertexAttribDivisorANGLE, VertexAttribDivisorARB, VertexAttribDivisorEXT, VertexAttribDivisorNV
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribDivisor(index: types::GLuint, divisor: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLuint) -> ()>(storage::VertexAttribDivisor.f)(index, divisor) }
/// Fallbacks: VertexAttribI1iEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI1i(index: types::GLuint, x: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLint) -> ()>(storage::VertexAttribI1i.f)(index, x) }
/// Fallbacks: VertexAttribI1ivEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI1iv(index: types::GLuint, v: *const types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLint) -> ()>(storage::VertexAttribI1iv.f)(index, v) }
/// Fallbacks: VertexAttribI1uiEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI1ui(index: types::GLuint, x: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLuint) -> ()>(storage::VertexAttribI1ui.f)(index, x) }
/// Fallbacks: VertexAttribI1uivEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI1uiv(index: types::GLuint, v: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLuint) -> ()>(storage::VertexAttribI1uiv.f)(index, v) }
/// Fallbacks: VertexAttribI2iEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI2i(index: types::GLuint, x: types::GLint, y: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLint, types::GLint) -> ()>(storage::VertexAttribI2i.f)(index, x, y) }
/// Fallbacks: VertexAttribI2ivEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI2iv(index: types::GLuint, v: *const types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLint) -> ()>(storage::VertexAttribI2iv.f)(index, v) }
/// Fallbacks: VertexAttribI2uiEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI2ui(index: types::GLuint, x: types::GLuint, y: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLuint, types::GLuint) -> ()>(storage::VertexAttribI2ui.f)(index, x, y) }
/// Fallbacks: VertexAttribI2uivEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI2uiv(index: types::GLuint, v: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLuint) -> ()>(storage::VertexAttribI2uiv.f)(index, v) }
/// Fallbacks: VertexAttribI3iEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI3i(index: types::GLuint, x: types::GLint, y: types::GLint, z: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLint, types::GLint, types::GLint) -> ()>(storage::VertexAttribI3i.f)(index, x, y, z) }
/// Fallbacks: VertexAttribI3ivEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI3iv(index: types::GLuint, v: *const types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLint) -> ()>(storage::VertexAttribI3iv.f)(index, v) }
/// Fallbacks: VertexAttribI3uiEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI3ui(index: types::GLuint, x: types::GLuint, y: types::GLuint, z: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLuint, types::GLuint, types::GLuint) -> ()>(storage::VertexAttribI3ui.f)(index, x, y, z) }
/// Fallbacks: VertexAttribI3uivEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI3uiv(index: types::GLuint, v: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLuint) -> ()>(storage::VertexAttribI3uiv.f)(index, v) }
/// Fallbacks: VertexAttribI4bvEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI4bv(index: types::GLuint, v: *const types::GLbyte) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLbyte) -> ()>(storage::VertexAttribI4bv.f)(index, v) }
/// Fallbacks: VertexAttribI4iEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI4i(index: types::GLuint, x: types::GLint, y: types::GLint, z: types::GLint, w: types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLint, types::GLint, types::GLint, types::GLint) -> ()>(storage::VertexAttribI4i.f)(index, x, y, z, w) }
/// Fallbacks: VertexAttribI4ivEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI4iv(index: types::GLuint, v: *const types::GLint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLint) -> ()>(storage::VertexAttribI4iv.f)(index, v) }
/// Fallbacks: VertexAttribI4svEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI4sv(index: types::GLuint, v: *const types::GLshort) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLshort) -> ()>(storage::VertexAttribI4sv.f)(index, v) }
/// Fallbacks: VertexAttribI4ubvEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI4ubv(index: types::GLuint, v: *const types::GLubyte) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLubyte) -> ()>(storage::VertexAttribI4ubv.f)(index, v) }
/// Fallbacks: VertexAttribI4uiEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI4ui(index: types::GLuint, x: types::GLuint, y: types::GLuint, z: types::GLuint, w: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLuint, types::GLuint, types::GLuint, types::GLuint) -> ()>(storage::VertexAttribI4ui.f)(index, x, y, z, w) }
/// Fallbacks: VertexAttribI4uivEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI4uiv(index: types::GLuint, v: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLuint) -> ()>(storage::VertexAttribI4uiv.f)(index, v) }
/// Fallbacks: VertexAttribI4usvEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribI4usv(index: types::GLuint, v: *const types::GLushort) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, *const types::GLushort) -> ()>(storage::VertexAttribI4usv.f)(index, v) }
/// Fallbacks: VertexAttribIPointerEXT
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribIPointer(index: types::GLuint, size: types::GLint, type_: types::GLenum, stride: types::GLsizei, pointer: *const __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLint, types::GLenum, types::GLsizei, *const __gl_imports::raw::c_void) -> ()>(storage::VertexAttribIPointer.f)(index, size, type_, stride, pointer) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribP1ui(index: types::GLuint, type_: types::GLenum, normalized: types::GLboolean, value: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, types::GLboolean, types::GLuint) -> ()>(storage::VertexAttribP1ui.f)(index, type_, normalized, value) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribP1uiv(index: types::GLuint, type_: types::GLenum, normalized: types::GLboolean, value: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, types::GLboolean, *const types::GLuint) -> ()>(storage::VertexAttribP1uiv.f)(index, type_, normalized, value) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribP2ui(index: types::GLuint, type_: types::GLenum, normalized: types::GLboolean, value: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, types::GLboolean, types::GLuint) -> ()>(storage::VertexAttribP2ui.f)(index, type_, normalized, value) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribP2uiv(index: types::GLuint, type_: types::GLenum, normalized: types::GLboolean, value: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, types::GLboolean, *const types::GLuint) -> ()>(storage::VertexAttribP2uiv.f)(index, type_, normalized, value) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribP3ui(index: types::GLuint, type_: types::GLenum, normalized: types::GLboolean, value: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, types::GLboolean, types::GLuint) -> ()>(storage::VertexAttribP3ui.f)(index, type_, normalized, value) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribP3uiv(index: types::GLuint, type_: types::GLenum, normalized: types::GLboolean, value: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, types::GLboolean, *const types::GLuint) -> ()>(storage::VertexAttribP3uiv.f)(index, type_, normalized, value) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribP4ui(index: types::GLuint, type_: types::GLenum, normalized: types::GLboolean, value: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, types::GLboolean, types::GLuint) -> ()>(storage::VertexAttribP4ui.f)(index, type_, normalized, value) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribP4uiv(index: types::GLuint, type_: types::GLenum, normalized: types::GLboolean, value: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLenum, types::GLboolean, *const types::GLuint) -> ()>(storage::VertexAttribP4uiv.f)(index, type_, normalized, value) }
/// Fallbacks: VertexAttribPointerARB
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexAttribPointer(index: types::GLuint, size: types::GLint, type_: types::GLenum, normalized: types::GLboolean, stride: types::GLsizei, pointer: *const __gl_imports::raw::c_void) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLuint, types::GLint, types::GLenum, types::GLboolean, types::GLsizei, *const __gl_imports::raw::c_void) -> ()>(storage::VertexAttribPointer.f)(index, size, type_, normalized, stride, pointer) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexP2ui(type_: types::GLenum, value: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::VertexP2ui.f)(type_, value) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexP2uiv(type_: types::GLenum, value: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *const types::GLuint) -> ()>(storage::VertexP2uiv.f)(type_, value) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexP3ui(type_: types::GLenum, value: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::VertexP3ui.f)(type_, value) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexP3uiv(type_: types::GLenum, value: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *const types::GLuint) -> ()>(storage::VertexP3uiv.f)(type_, value) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexP4ui(type_: types::GLenum, value: types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, types::GLuint) -> ()>(storage::VertexP4ui.f)(type_, value) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn VertexP4uiv(type_: types::GLenum, value: *const types::GLuint) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLenum, *const types::GLuint) -> ()>(storage::VertexP4uiv.f)(type_, value) }
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn Viewport(x: types::GLint, y: types::GLint, width: types::GLsizei, height: types::GLsizei) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLint, types::GLint, types::GLsizei, types::GLsizei) -> ()>(storage::Viewport.f)(x, y, width, height) }
/// Fallbacks: WaitSyncAPPLE
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn WaitSync(sync: types::GLsync, flags: types::GLbitfield, timeout: types::GLuint64) -> () { __gl_imports::mem::transmute::<_, extern "system" fn(types::GLsync, types::GLbitfield, types::GLuint64) -> ()>(storage::WaitSync.f)(sync, flags, timeout) }

        #[allow(missing_copy_implementations)]
        pub struct FnPtr {
            /// The function pointer that will be used when calling the function.
            f: *const __gl_imports::raw::c_void,
            /// True if the pointer points to a real function, false if points to a `panic!` fn.
            is_loaded: bool,
        }

        impl FnPtr {
            /// Creates a `FnPtr` from a load attempt.
            pub fn new(ptr: *const __gl_imports::raw::c_void) -> FnPtr {
                if ptr.is_null() {
                    FnPtr { f: missing_fn_panic as *const __gl_imports::raw::c_void, is_loaded: false }
                } else {
                    FnPtr { f: ptr, is_loaded: true }
                }
            }
        }
    
mod storage {
            #![allow(non_snake_case)]
            #![allow(non_upper_case_globals)]
            use super::__gl_imports::raw;
            use super::FnPtr;
pub static mut ActiveTexture: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut AttachShader: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BeginConditionalRender: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BeginQuery: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BeginTransformFeedback: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BindAttribLocation: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BindBuffer: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BindBufferBase: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BindBufferRange: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BindFragDataLocation: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BindFragDataLocationIndexed: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BindFramebuffer: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BindRenderbuffer: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BindSampler: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BindTexture: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BindVertexArray: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BlendColor: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BlendEquation: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BlendEquationSeparate: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BlendFunc: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BlendFuncSeparate: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BlitFramebuffer: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BufferData: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut BufferSubData: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut CheckFramebufferStatus: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ClampColor: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Clear: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ClearBufferfi: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ClearBufferfv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ClearBufferiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ClearBufferuiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ClearColor: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ClearDepth: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ClearStencil: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ClientWaitSync: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ColorMask: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ColorMaski: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ColorP3ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ColorP3uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ColorP4ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ColorP4uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut CompileShader: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut CompressedTexImage1D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut CompressedTexImage2D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut CompressedTexImage3D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut CompressedTexSubImage1D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut CompressedTexSubImage2D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut CompressedTexSubImage3D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut CopyBufferSubData: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut CopyTexImage1D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut CopyTexImage2D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut CopyTexSubImage1D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut CopyTexSubImage2D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut CopyTexSubImage3D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut CreateProgram: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut CreateShader: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut CullFace: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DeleteBuffers: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DeleteFramebuffers: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DeleteProgram: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DeleteQueries: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DeleteRenderbuffers: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DeleteSamplers: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DeleteShader: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DeleteSync: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DeleteTextures: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DeleteVertexArrays: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DepthFunc: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DepthMask: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DepthRange: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DetachShader: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Disable: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DisableVertexAttribArray: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Disablei: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DrawArrays: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DrawArraysInstanced: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DrawBuffer: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DrawBuffers: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DrawElements: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DrawElementsBaseVertex: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DrawElementsInstanced: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DrawElementsInstancedBaseVertex: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DrawRangeElements: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut DrawRangeElementsBaseVertex: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Enable: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut EnableVertexAttribArray: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Enablei: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut EndConditionalRender: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut EndQuery: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut EndTransformFeedback: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut FenceSync: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Finish: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Flush: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut FlushMappedBufferRange: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut FramebufferRenderbuffer: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut FramebufferTexture: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut FramebufferTexture1D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut FramebufferTexture2D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut FramebufferTexture3D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut FramebufferTextureLayer: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut FrontFace: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GenBuffers: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GenFramebuffers: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GenQueries: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GenRenderbuffers: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GenSamplers: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GenTextures: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GenVertexArrays: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GenerateMipmap: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetActiveAttrib: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetActiveUniform: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetActiveUniformBlockName: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetActiveUniformBlockiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetActiveUniformName: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetActiveUniformsiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetAttachedShaders: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetAttribLocation: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetBooleani_v: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetBooleanv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetBufferParameteri64v: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetBufferParameteriv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetBufferPointerv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetBufferSubData: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetCompressedTexImage: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetDoublev: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetError: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetFloatv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetFragDataIndex: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetFragDataLocation: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetFramebufferAttachmentParameteriv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetInteger64i_v: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetInteger64v: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetIntegeri_v: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetIntegerv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetMultisamplefv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetProgramInfoLog: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetProgramiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetQueryObjecti64v: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetQueryObjectiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetQueryObjectui64v: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetQueryObjectuiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetQueryiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetRenderbufferParameteriv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetSamplerParameterIiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetSamplerParameterIuiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetSamplerParameterfv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetSamplerParameteriv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetShaderInfoLog: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetShaderSource: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetShaderiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetString: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetStringi: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetSynciv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetTexImage: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetTexLevelParameterfv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetTexLevelParameteriv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetTexParameterIiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetTexParameterIuiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetTexParameterfv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetTexParameteriv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetTransformFeedbackVarying: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetUniformBlockIndex: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetUniformIndices: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetUniformLocation: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetUniformfv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetUniformiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetUniformuiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetVertexAttribIiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetVertexAttribIuiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetVertexAttribPointerv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetVertexAttribdv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetVertexAttribfv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut GetVertexAttribiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Hint: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut IsBuffer: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut IsEnabled: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut IsEnabledi: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut IsFramebuffer: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut IsProgram: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut IsQuery: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut IsRenderbuffer: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut IsSampler: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut IsShader: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut IsSync: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut IsTexture: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut IsVertexArray: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut LineWidth: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut LinkProgram: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut LogicOp: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut MapBuffer: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut MapBufferRange: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut MultiDrawArrays: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut MultiDrawElements: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut MultiDrawElementsBaseVertex: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut MultiTexCoordP1ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut MultiTexCoordP1uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut MultiTexCoordP2ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut MultiTexCoordP2uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut MultiTexCoordP3ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut MultiTexCoordP3uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut MultiTexCoordP4ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut MultiTexCoordP4uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut NormalP3ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut NormalP3uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut PixelStoref: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut PixelStorei: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut PointParameterf: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut PointParameterfv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut PointParameteri: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut PointParameteriv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut PointSize: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut PolygonMode: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut PolygonOffset: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut PrimitiveRestartIndex: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ProvokingVertex: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut QueryCounter: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ReadBuffer: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ReadPixels: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut RenderbufferStorage: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut RenderbufferStorageMultisample: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut SampleCoverage: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut SampleMaski: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut SamplerParameterIiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut SamplerParameterIuiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut SamplerParameterf: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut SamplerParameterfv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut SamplerParameteri: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut SamplerParameteriv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Scissor: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut SecondaryColorP3ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut SecondaryColorP3uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ShaderSource: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut StencilFunc: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut StencilFuncSeparate: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut StencilMask: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut StencilMaskSeparate: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut StencilOp: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut StencilOpSeparate: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexBuffer: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexCoordP1ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexCoordP1uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexCoordP2ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexCoordP2uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexCoordP3ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexCoordP3uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexCoordP4ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexCoordP4uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexImage1D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexImage2D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexImage2DMultisample: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexImage3D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexImage3DMultisample: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexParameterIiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexParameterIuiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexParameterf: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexParameterfv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexParameteri: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexParameteriv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexSubImage1D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexSubImage2D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TexSubImage3D: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut TransformFeedbackVaryings: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform1f: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform1fv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform1i: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform1iv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform1ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform1uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform2f: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform2fv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform2i: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform2iv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform2ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform2uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform3f: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform3fv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform3i: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform3iv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform3ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform3uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform4f: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform4fv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform4i: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform4iv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform4ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Uniform4uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut UniformBlockBinding: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut UniformMatrix2fv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut UniformMatrix2x3fv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut UniformMatrix2x4fv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut UniformMatrix3fv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut UniformMatrix3x2fv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut UniformMatrix3x4fv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut UniformMatrix4fv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut UniformMatrix4x2fv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut UniformMatrix4x3fv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut UnmapBuffer: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut UseProgram: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut ValidateProgram: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib1d: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib1dv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib1f: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib1fv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib1s: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib1sv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib2d: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib2dv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib2f: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib2fv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib2s: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib2sv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib3d: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib3dv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib3f: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib3fv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib3s: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib3sv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib4Nbv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib4Niv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib4Nsv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib4Nub: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib4Nubv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib4Nuiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib4Nusv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib4bv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib4d: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib4dv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib4f: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib4fv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib4iv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib4s: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib4sv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib4ubv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib4uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttrib4usv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribDivisor: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI1i: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI1iv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI1ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI1uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI2i: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI2iv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI2ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI2uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI3i: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI3iv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI3ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI3uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI4bv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI4i: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI4iv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI4sv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI4ubv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI4ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI4uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribI4usv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribIPointer: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribP1ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribP1uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribP2ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribP2uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribP3ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribP3uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribP4ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribP4uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexAttribPointer: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexP2ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexP2uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexP3ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexP3uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexP4ui: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut VertexP4uiv: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut Viewport: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
pub static mut WaitSync: FnPtr = FnPtr {
                f: super::missing_fn_panic as *const raw::c_void,
                is_loaded: false
            };
}

            #[allow(non_snake_case)]
            pub mod ActiveTexture {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ActiveTexture.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ActiveTexture = FnPtr::new(metaloadfn(&mut loadfn, "glActiveTexture", &["glActiveTextureARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod AttachShader {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::AttachShader.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::AttachShader = FnPtr::new(metaloadfn(&mut loadfn, "glAttachShader", &["glAttachObjectARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BeginConditionalRender {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BeginConditionalRender.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BeginConditionalRender = FnPtr::new(metaloadfn(&mut loadfn, "glBeginConditionalRender", &["glBeginConditionalRenderNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BeginQuery {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BeginQuery.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BeginQuery = FnPtr::new(metaloadfn(&mut loadfn, "glBeginQuery", &["glBeginQueryARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BeginTransformFeedback {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BeginTransformFeedback.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BeginTransformFeedback = FnPtr::new(metaloadfn(&mut loadfn, "glBeginTransformFeedback", &["glBeginTransformFeedbackEXT", "glBeginTransformFeedbackNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BindAttribLocation {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BindAttribLocation.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BindAttribLocation = FnPtr::new(metaloadfn(&mut loadfn, "glBindAttribLocation", &["glBindAttribLocationARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BindBuffer {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BindBuffer.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BindBuffer = FnPtr::new(metaloadfn(&mut loadfn, "glBindBuffer", &["glBindBufferARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BindBufferBase {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BindBufferBase.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BindBufferBase = FnPtr::new(metaloadfn(&mut loadfn, "glBindBufferBase", &["glBindBufferBaseEXT", "glBindBufferBaseNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BindBufferRange {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BindBufferRange.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BindBufferRange = FnPtr::new(metaloadfn(&mut loadfn, "glBindBufferRange", &["glBindBufferRangeEXT", "glBindBufferRangeNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BindFragDataLocation {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BindFragDataLocation.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BindFragDataLocation = FnPtr::new(metaloadfn(&mut loadfn, "glBindFragDataLocation", &["glBindFragDataLocationEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BindFragDataLocationIndexed {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BindFragDataLocationIndexed.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BindFragDataLocationIndexed = FnPtr::new(metaloadfn(&mut loadfn, "glBindFragDataLocationIndexed", &["glBindFragDataLocationIndexedEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BindFramebuffer {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BindFramebuffer.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BindFramebuffer = FnPtr::new(metaloadfn(&mut loadfn, "glBindFramebuffer", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BindRenderbuffer {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BindRenderbuffer.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BindRenderbuffer = FnPtr::new(metaloadfn(&mut loadfn, "glBindRenderbuffer", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BindSampler {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BindSampler.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BindSampler = FnPtr::new(metaloadfn(&mut loadfn, "glBindSampler", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BindTexture {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BindTexture.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BindTexture = FnPtr::new(metaloadfn(&mut loadfn, "glBindTexture", &["glBindTextureEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BindVertexArray {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BindVertexArray.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BindVertexArray = FnPtr::new(metaloadfn(&mut loadfn, "glBindVertexArray", &["glBindVertexArrayOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BlendColor {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BlendColor.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BlendColor = FnPtr::new(metaloadfn(&mut loadfn, "glBlendColor", &["glBlendColorEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BlendEquation {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BlendEquation.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BlendEquation = FnPtr::new(metaloadfn(&mut loadfn, "glBlendEquation", &["glBlendEquationEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BlendEquationSeparate {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BlendEquationSeparate.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BlendEquationSeparate = FnPtr::new(metaloadfn(&mut loadfn, "glBlendEquationSeparate", &["glBlendEquationSeparateEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BlendFunc {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BlendFunc.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BlendFunc = FnPtr::new(metaloadfn(&mut loadfn, "glBlendFunc", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BlendFuncSeparate {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BlendFuncSeparate.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BlendFuncSeparate = FnPtr::new(metaloadfn(&mut loadfn, "glBlendFuncSeparate", &["glBlendFuncSeparateEXT", "glBlendFuncSeparateINGR"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BlitFramebuffer {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BlitFramebuffer.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BlitFramebuffer = FnPtr::new(metaloadfn(&mut loadfn, "glBlitFramebuffer", &["glBlitFramebufferEXT", "glBlitFramebufferNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BufferData {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BufferData.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BufferData = FnPtr::new(metaloadfn(&mut loadfn, "glBufferData", &["glBufferDataARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod BufferSubData {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::BufferSubData.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::BufferSubData = FnPtr::new(metaloadfn(&mut loadfn, "glBufferSubData", &["glBufferSubDataARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod CheckFramebufferStatus {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::CheckFramebufferStatus.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::CheckFramebufferStatus = FnPtr::new(metaloadfn(&mut loadfn, "glCheckFramebufferStatus", &["glCheckFramebufferStatusEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ClampColor {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ClampColor.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ClampColor = FnPtr::new(metaloadfn(&mut loadfn, "glClampColor", &["glClampColorARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Clear {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Clear.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Clear = FnPtr::new(metaloadfn(&mut loadfn, "glClear", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ClearBufferfi {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ClearBufferfi.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ClearBufferfi = FnPtr::new(metaloadfn(&mut loadfn, "glClearBufferfi", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ClearBufferfv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ClearBufferfv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ClearBufferfv = FnPtr::new(metaloadfn(&mut loadfn, "glClearBufferfv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ClearBufferiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ClearBufferiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ClearBufferiv = FnPtr::new(metaloadfn(&mut loadfn, "glClearBufferiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ClearBufferuiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ClearBufferuiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ClearBufferuiv = FnPtr::new(metaloadfn(&mut loadfn, "glClearBufferuiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ClearColor {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ClearColor.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ClearColor = FnPtr::new(metaloadfn(&mut loadfn, "glClearColor", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ClearDepth {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ClearDepth.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ClearDepth = FnPtr::new(metaloadfn(&mut loadfn, "glClearDepth", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ClearStencil {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ClearStencil.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ClearStencil = FnPtr::new(metaloadfn(&mut loadfn, "glClearStencil", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ClientWaitSync {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ClientWaitSync.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ClientWaitSync = FnPtr::new(metaloadfn(&mut loadfn, "glClientWaitSync", &["glClientWaitSyncAPPLE"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ColorMask {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ColorMask.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ColorMask = FnPtr::new(metaloadfn(&mut loadfn, "glColorMask", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ColorMaski {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ColorMaski.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ColorMaski = FnPtr::new(metaloadfn(&mut loadfn, "glColorMaski", &["glColorMaskIndexedEXT", "glColorMaskiEXT", "glColorMaskiOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ColorP3ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ColorP3ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ColorP3ui = FnPtr::new(metaloadfn(&mut loadfn, "glColorP3ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ColorP3uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ColorP3uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ColorP3uiv = FnPtr::new(metaloadfn(&mut loadfn, "glColorP3uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ColorP4ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ColorP4ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ColorP4ui = FnPtr::new(metaloadfn(&mut loadfn, "glColorP4ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ColorP4uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ColorP4uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ColorP4uiv = FnPtr::new(metaloadfn(&mut loadfn, "glColorP4uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod CompileShader {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::CompileShader.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::CompileShader = FnPtr::new(metaloadfn(&mut loadfn, "glCompileShader", &["glCompileShaderARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod CompressedTexImage1D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::CompressedTexImage1D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::CompressedTexImage1D = FnPtr::new(metaloadfn(&mut loadfn, "glCompressedTexImage1D", &["glCompressedTexImage1DARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod CompressedTexImage2D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::CompressedTexImage2D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::CompressedTexImage2D = FnPtr::new(metaloadfn(&mut loadfn, "glCompressedTexImage2D", &["glCompressedTexImage2DARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod CompressedTexImage3D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::CompressedTexImage3D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::CompressedTexImage3D = FnPtr::new(metaloadfn(&mut loadfn, "glCompressedTexImage3D", &["glCompressedTexImage3DARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod CompressedTexSubImage1D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::CompressedTexSubImage1D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::CompressedTexSubImage1D = FnPtr::new(metaloadfn(&mut loadfn, "glCompressedTexSubImage1D", &["glCompressedTexSubImage1DARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod CompressedTexSubImage2D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::CompressedTexSubImage2D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::CompressedTexSubImage2D = FnPtr::new(metaloadfn(&mut loadfn, "glCompressedTexSubImage2D", &["glCompressedTexSubImage2DARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod CompressedTexSubImage3D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::CompressedTexSubImage3D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::CompressedTexSubImage3D = FnPtr::new(metaloadfn(&mut loadfn, "glCompressedTexSubImage3D", &["glCompressedTexSubImage3DARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod CopyBufferSubData {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::CopyBufferSubData.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::CopyBufferSubData = FnPtr::new(metaloadfn(&mut loadfn, "glCopyBufferSubData", &["glCopyBufferSubDataNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod CopyTexImage1D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::CopyTexImage1D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::CopyTexImage1D = FnPtr::new(metaloadfn(&mut loadfn, "glCopyTexImage1D", &["glCopyTexImage1DEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod CopyTexImage2D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::CopyTexImage2D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::CopyTexImage2D = FnPtr::new(metaloadfn(&mut loadfn, "glCopyTexImage2D", &["glCopyTexImage2DEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod CopyTexSubImage1D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::CopyTexSubImage1D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::CopyTexSubImage1D = FnPtr::new(metaloadfn(&mut loadfn, "glCopyTexSubImage1D", &["glCopyTexSubImage1DEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod CopyTexSubImage2D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::CopyTexSubImage2D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::CopyTexSubImage2D = FnPtr::new(metaloadfn(&mut loadfn, "glCopyTexSubImage2D", &["glCopyTexSubImage2DEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod CopyTexSubImage3D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::CopyTexSubImage3D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::CopyTexSubImage3D = FnPtr::new(metaloadfn(&mut loadfn, "glCopyTexSubImage3D", &["glCopyTexSubImage3DEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod CreateProgram {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::CreateProgram.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::CreateProgram = FnPtr::new(metaloadfn(&mut loadfn, "glCreateProgram", &["glCreateProgramObjectARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod CreateShader {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::CreateShader.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::CreateShader = FnPtr::new(metaloadfn(&mut loadfn, "glCreateShader", &["glCreateShaderObjectARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod CullFace {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::CullFace.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::CullFace = FnPtr::new(metaloadfn(&mut loadfn, "glCullFace", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DeleteBuffers {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DeleteBuffers.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DeleteBuffers = FnPtr::new(metaloadfn(&mut loadfn, "glDeleteBuffers", &["glDeleteBuffersARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DeleteFramebuffers {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DeleteFramebuffers.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DeleteFramebuffers = FnPtr::new(metaloadfn(&mut loadfn, "glDeleteFramebuffers", &["glDeleteFramebuffersEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DeleteProgram {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DeleteProgram.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DeleteProgram = FnPtr::new(metaloadfn(&mut loadfn, "glDeleteProgram", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DeleteQueries {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DeleteQueries.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DeleteQueries = FnPtr::new(metaloadfn(&mut loadfn, "glDeleteQueries", &["glDeleteQueriesARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DeleteRenderbuffers {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DeleteRenderbuffers.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DeleteRenderbuffers = FnPtr::new(metaloadfn(&mut loadfn, "glDeleteRenderbuffers", &["glDeleteRenderbuffersEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DeleteSamplers {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DeleteSamplers.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DeleteSamplers = FnPtr::new(metaloadfn(&mut loadfn, "glDeleteSamplers", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DeleteShader {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DeleteShader.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DeleteShader = FnPtr::new(metaloadfn(&mut loadfn, "glDeleteShader", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DeleteSync {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DeleteSync.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DeleteSync = FnPtr::new(metaloadfn(&mut loadfn, "glDeleteSync", &["glDeleteSyncAPPLE"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DeleteTextures {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DeleteTextures.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DeleteTextures = FnPtr::new(metaloadfn(&mut loadfn, "glDeleteTextures", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DeleteVertexArrays {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DeleteVertexArrays.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DeleteVertexArrays = FnPtr::new(metaloadfn(&mut loadfn, "glDeleteVertexArrays", &["glDeleteVertexArraysAPPLE", "glDeleteVertexArraysOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DepthFunc {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DepthFunc.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DepthFunc = FnPtr::new(metaloadfn(&mut loadfn, "glDepthFunc", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DepthMask {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DepthMask.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DepthMask = FnPtr::new(metaloadfn(&mut loadfn, "glDepthMask", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DepthRange {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DepthRange.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DepthRange = FnPtr::new(metaloadfn(&mut loadfn, "glDepthRange", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DetachShader {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DetachShader.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DetachShader = FnPtr::new(metaloadfn(&mut loadfn, "glDetachShader", &["glDetachObjectARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Disable {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Disable.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Disable = FnPtr::new(metaloadfn(&mut loadfn, "glDisable", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DisableVertexAttribArray {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DisableVertexAttribArray.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DisableVertexAttribArray = FnPtr::new(metaloadfn(&mut loadfn, "glDisableVertexAttribArray", &["glDisableVertexAttribArrayARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Disablei {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Disablei.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Disablei = FnPtr::new(metaloadfn(&mut loadfn, "glDisablei", &["glDisableIndexedEXT", "glDisableiEXT", "glDisableiNV", "glDisableiOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DrawArrays {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DrawArrays.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DrawArrays = FnPtr::new(metaloadfn(&mut loadfn, "glDrawArrays", &["glDrawArraysEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DrawArraysInstanced {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DrawArraysInstanced.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DrawArraysInstanced = FnPtr::new(metaloadfn(&mut loadfn, "glDrawArraysInstanced", &["glDrawArraysInstancedANGLE", "glDrawArraysInstancedARB", "glDrawArraysInstancedEXT", "glDrawArraysInstancedNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DrawBuffer {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DrawBuffer.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DrawBuffer = FnPtr::new(metaloadfn(&mut loadfn, "glDrawBuffer", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DrawBuffers {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DrawBuffers.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DrawBuffers = FnPtr::new(metaloadfn(&mut loadfn, "glDrawBuffers", &["glDrawBuffersARB", "glDrawBuffersATI", "glDrawBuffersEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DrawElements {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DrawElements.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DrawElements = FnPtr::new(metaloadfn(&mut loadfn, "glDrawElements", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DrawElementsBaseVertex {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DrawElementsBaseVertex.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DrawElementsBaseVertex = FnPtr::new(metaloadfn(&mut loadfn, "glDrawElementsBaseVertex", &["glDrawElementsBaseVertexEXT", "glDrawElementsBaseVertexOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DrawElementsInstanced {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DrawElementsInstanced.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DrawElementsInstanced = FnPtr::new(metaloadfn(&mut loadfn, "glDrawElementsInstanced", &["glDrawElementsInstancedANGLE", "glDrawElementsInstancedARB", "glDrawElementsInstancedEXT", "glDrawElementsInstancedNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DrawElementsInstancedBaseVertex {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DrawElementsInstancedBaseVertex.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DrawElementsInstancedBaseVertex = FnPtr::new(metaloadfn(&mut loadfn, "glDrawElementsInstancedBaseVertex", &["glDrawElementsInstancedBaseVertexEXT", "glDrawElementsInstancedBaseVertexOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DrawRangeElements {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DrawRangeElements.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DrawRangeElements = FnPtr::new(metaloadfn(&mut loadfn, "glDrawRangeElements", &["glDrawRangeElementsEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod DrawRangeElementsBaseVertex {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::DrawRangeElementsBaseVertex.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::DrawRangeElementsBaseVertex = FnPtr::new(metaloadfn(&mut loadfn, "glDrawRangeElementsBaseVertex", &["glDrawRangeElementsBaseVertexEXT", "glDrawRangeElementsBaseVertexOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Enable {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Enable.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Enable = FnPtr::new(metaloadfn(&mut loadfn, "glEnable", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod EnableVertexAttribArray {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::EnableVertexAttribArray.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::EnableVertexAttribArray = FnPtr::new(metaloadfn(&mut loadfn, "glEnableVertexAttribArray", &["glEnableVertexAttribArrayARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Enablei {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Enablei.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Enablei = FnPtr::new(metaloadfn(&mut loadfn, "glEnablei", &["glEnableIndexedEXT", "glEnableiEXT", "glEnableiNV", "glEnableiOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod EndConditionalRender {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::EndConditionalRender.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::EndConditionalRender = FnPtr::new(metaloadfn(&mut loadfn, "glEndConditionalRender", &["glEndConditionalRenderNV", "glEndConditionalRenderNVX"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod EndQuery {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::EndQuery.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::EndQuery = FnPtr::new(metaloadfn(&mut loadfn, "glEndQuery", &["glEndQueryARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod EndTransformFeedback {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::EndTransformFeedback.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::EndTransformFeedback = FnPtr::new(metaloadfn(&mut loadfn, "glEndTransformFeedback", &["glEndTransformFeedbackEXT", "glEndTransformFeedbackNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod FenceSync {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::FenceSync.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::FenceSync = FnPtr::new(metaloadfn(&mut loadfn, "glFenceSync", &["glFenceSyncAPPLE"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Finish {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Finish.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Finish = FnPtr::new(metaloadfn(&mut loadfn, "glFinish", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Flush {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Flush.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Flush = FnPtr::new(metaloadfn(&mut loadfn, "glFlush", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod FlushMappedBufferRange {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::FlushMappedBufferRange.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::FlushMappedBufferRange = FnPtr::new(metaloadfn(&mut loadfn, "glFlushMappedBufferRange", &["glFlushMappedBufferRangeAPPLE", "glFlushMappedBufferRangeEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod FramebufferRenderbuffer {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::FramebufferRenderbuffer.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::FramebufferRenderbuffer = FnPtr::new(metaloadfn(&mut loadfn, "glFramebufferRenderbuffer", &["glFramebufferRenderbufferEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod FramebufferTexture {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::FramebufferTexture.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::FramebufferTexture = FnPtr::new(metaloadfn(&mut loadfn, "glFramebufferTexture", &["glFramebufferTextureARB", "glFramebufferTextureEXT", "glFramebufferTextureOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod FramebufferTexture1D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::FramebufferTexture1D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::FramebufferTexture1D = FnPtr::new(metaloadfn(&mut loadfn, "glFramebufferTexture1D", &["glFramebufferTexture1DEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod FramebufferTexture2D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::FramebufferTexture2D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::FramebufferTexture2D = FnPtr::new(metaloadfn(&mut loadfn, "glFramebufferTexture2D", &["glFramebufferTexture2DEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod FramebufferTexture3D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::FramebufferTexture3D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::FramebufferTexture3D = FnPtr::new(metaloadfn(&mut loadfn, "glFramebufferTexture3D", &["glFramebufferTexture3DEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod FramebufferTextureLayer {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::FramebufferTextureLayer.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::FramebufferTextureLayer = FnPtr::new(metaloadfn(&mut loadfn, "glFramebufferTextureLayer", &["glFramebufferTextureLayerARB", "glFramebufferTextureLayerEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod FrontFace {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::FrontFace.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::FrontFace = FnPtr::new(metaloadfn(&mut loadfn, "glFrontFace", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GenBuffers {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GenBuffers.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GenBuffers = FnPtr::new(metaloadfn(&mut loadfn, "glGenBuffers", &["glGenBuffersARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GenFramebuffers {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GenFramebuffers.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GenFramebuffers = FnPtr::new(metaloadfn(&mut loadfn, "glGenFramebuffers", &["glGenFramebuffersEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GenQueries {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GenQueries.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GenQueries = FnPtr::new(metaloadfn(&mut loadfn, "glGenQueries", &["glGenQueriesARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GenRenderbuffers {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GenRenderbuffers.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GenRenderbuffers = FnPtr::new(metaloadfn(&mut loadfn, "glGenRenderbuffers", &["glGenRenderbuffersEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GenSamplers {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GenSamplers.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GenSamplers = FnPtr::new(metaloadfn(&mut loadfn, "glGenSamplers", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GenTextures {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GenTextures.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GenTextures = FnPtr::new(metaloadfn(&mut loadfn, "glGenTextures", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GenVertexArrays {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GenVertexArrays.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GenVertexArrays = FnPtr::new(metaloadfn(&mut loadfn, "glGenVertexArrays", &["glGenVertexArraysAPPLE", "glGenVertexArraysOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GenerateMipmap {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GenerateMipmap.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GenerateMipmap = FnPtr::new(metaloadfn(&mut loadfn, "glGenerateMipmap", &["glGenerateMipmapEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetActiveAttrib {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetActiveAttrib.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetActiveAttrib = FnPtr::new(metaloadfn(&mut loadfn, "glGetActiveAttrib", &["glGetActiveAttribARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetActiveUniform {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetActiveUniform.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetActiveUniform = FnPtr::new(metaloadfn(&mut loadfn, "glGetActiveUniform", &["glGetActiveUniformARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetActiveUniformBlockName {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetActiveUniformBlockName.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetActiveUniformBlockName = FnPtr::new(metaloadfn(&mut loadfn, "glGetActiveUniformBlockName", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetActiveUniformBlockiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetActiveUniformBlockiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetActiveUniformBlockiv = FnPtr::new(metaloadfn(&mut loadfn, "glGetActiveUniformBlockiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetActiveUniformName {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetActiveUniformName.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetActiveUniformName = FnPtr::new(metaloadfn(&mut loadfn, "glGetActiveUniformName", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetActiveUniformsiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetActiveUniformsiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetActiveUniformsiv = FnPtr::new(metaloadfn(&mut loadfn, "glGetActiveUniformsiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetAttachedShaders {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetAttachedShaders.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetAttachedShaders = FnPtr::new(metaloadfn(&mut loadfn, "glGetAttachedShaders", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetAttribLocation {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetAttribLocation.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetAttribLocation = FnPtr::new(metaloadfn(&mut loadfn, "glGetAttribLocation", &["glGetAttribLocationARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetBooleani_v {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetBooleani_v.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetBooleani_v = FnPtr::new(metaloadfn(&mut loadfn, "glGetBooleani_v", &["glGetBooleanIndexedvEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetBooleanv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetBooleanv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetBooleanv = FnPtr::new(metaloadfn(&mut loadfn, "glGetBooleanv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetBufferParameteri64v {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetBufferParameteri64v.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetBufferParameteri64v = FnPtr::new(metaloadfn(&mut loadfn, "glGetBufferParameteri64v", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetBufferParameteriv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetBufferParameteriv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetBufferParameteriv = FnPtr::new(metaloadfn(&mut loadfn, "glGetBufferParameteriv", &["glGetBufferParameterivARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetBufferPointerv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetBufferPointerv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetBufferPointerv = FnPtr::new(metaloadfn(&mut loadfn, "glGetBufferPointerv", &["glGetBufferPointervARB", "glGetBufferPointervOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetBufferSubData {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetBufferSubData.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetBufferSubData = FnPtr::new(metaloadfn(&mut loadfn, "glGetBufferSubData", &["glGetBufferSubDataARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetCompressedTexImage {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetCompressedTexImage.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetCompressedTexImage = FnPtr::new(metaloadfn(&mut loadfn, "glGetCompressedTexImage", &["glGetCompressedTexImageARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetDoublev {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetDoublev.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetDoublev = FnPtr::new(metaloadfn(&mut loadfn, "glGetDoublev", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetError {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetError.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetError = FnPtr::new(metaloadfn(&mut loadfn, "glGetError", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetFloatv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetFloatv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetFloatv = FnPtr::new(metaloadfn(&mut loadfn, "glGetFloatv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetFragDataIndex {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetFragDataIndex.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetFragDataIndex = FnPtr::new(metaloadfn(&mut loadfn, "glGetFragDataIndex", &["glGetFragDataIndexEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetFragDataLocation {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetFragDataLocation.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetFragDataLocation = FnPtr::new(metaloadfn(&mut loadfn, "glGetFragDataLocation", &["glGetFragDataLocationEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetFramebufferAttachmentParameteriv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetFramebufferAttachmentParameteriv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetFramebufferAttachmentParameteriv = FnPtr::new(metaloadfn(&mut loadfn, "glGetFramebufferAttachmentParameteriv", &["glGetFramebufferAttachmentParameterivEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetInteger64i_v {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetInteger64i_v.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetInteger64i_v = FnPtr::new(metaloadfn(&mut loadfn, "glGetInteger64i_v", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetInteger64v {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetInteger64v.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetInteger64v = FnPtr::new(metaloadfn(&mut loadfn, "glGetInteger64v", &["glGetInteger64vAPPLE"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetIntegeri_v {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetIntegeri_v.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetIntegeri_v = FnPtr::new(metaloadfn(&mut loadfn, "glGetIntegeri_v", &["glGetIntegerIndexedvEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetIntegerv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetIntegerv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetIntegerv = FnPtr::new(metaloadfn(&mut loadfn, "glGetIntegerv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetMultisamplefv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetMultisamplefv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetMultisamplefv = FnPtr::new(metaloadfn(&mut loadfn, "glGetMultisamplefv", &["glGetMultisamplefvNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetProgramInfoLog {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetProgramInfoLog.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetProgramInfoLog = FnPtr::new(metaloadfn(&mut loadfn, "glGetProgramInfoLog", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetProgramiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetProgramiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetProgramiv = FnPtr::new(metaloadfn(&mut loadfn, "glGetProgramiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetQueryObjecti64v {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetQueryObjecti64v.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetQueryObjecti64v = FnPtr::new(metaloadfn(&mut loadfn, "glGetQueryObjecti64v", &["glGetQueryObjecti64vEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetQueryObjectiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetQueryObjectiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetQueryObjectiv = FnPtr::new(metaloadfn(&mut loadfn, "glGetQueryObjectiv", &["glGetQueryObjectivARB", "glGetQueryObjectivEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetQueryObjectui64v {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetQueryObjectui64v.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetQueryObjectui64v = FnPtr::new(metaloadfn(&mut loadfn, "glGetQueryObjectui64v", &["glGetQueryObjectui64vEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetQueryObjectuiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetQueryObjectuiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetQueryObjectuiv = FnPtr::new(metaloadfn(&mut loadfn, "glGetQueryObjectuiv", &["glGetQueryObjectuivARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetQueryiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetQueryiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetQueryiv = FnPtr::new(metaloadfn(&mut loadfn, "glGetQueryiv", &["glGetQueryivARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetRenderbufferParameteriv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetRenderbufferParameteriv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetRenderbufferParameteriv = FnPtr::new(metaloadfn(&mut loadfn, "glGetRenderbufferParameteriv", &["glGetRenderbufferParameterivEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetSamplerParameterIiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetSamplerParameterIiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetSamplerParameterIiv = FnPtr::new(metaloadfn(&mut loadfn, "glGetSamplerParameterIiv", &["glGetSamplerParameterIivEXT", "glGetSamplerParameterIivOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetSamplerParameterIuiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetSamplerParameterIuiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetSamplerParameterIuiv = FnPtr::new(metaloadfn(&mut loadfn, "glGetSamplerParameterIuiv", &["glGetSamplerParameterIuivEXT", "glGetSamplerParameterIuivOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetSamplerParameterfv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetSamplerParameterfv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetSamplerParameterfv = FnPtr::new(metaloadfn(&mut loadfn, "glGetSamplerParameterfv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetSamplerParameteriv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetSamplerParameteriv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetSamplerParameteriv = FnPtr::new(metaloadfn(&mut loadfn, "glGetSamplerParameteriv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetShaderInfoLog {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetShaderInfoLog.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetShaderInfoLog = FnPtr::new(metaloadfn(&mut loadfn, "glGetShaderInfoLog", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetShaderSource {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetShaderSource.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetShaderSource = FnPtr::new(metaloadfn(&mut loadfn, "glGetShaderSource", &["glGetShaderSourceARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetShaderiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetShaderiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetShaderiv = FnPtr::new(metaloadfn(&mut loadfn, "glGetShaderiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetString {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetString.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetString = FnPtr::new(metaloadfn(&mut loadfn, "glGetString", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetStringi {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetStringi.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetStringi = FnPtr::new(metaloadfn(&mut loadfn, "glGetStringi", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetSynciv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetSynciv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetSynciv = FnPtr::new(metaloadfn(&mut loadfn, "glGetSynciv", &["glGetSyncivAPPLE"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetTexImage {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetTexImage.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetTexImage = FnPtr::new(metaloadfn(&mut loadfn, "glGetTexImage", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetTexLevelParameterfv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetTexLevelParameterfv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetTexLevelParameterfv = FnPtr::new(metaloadfn(&mut loadfn, "glGetTexLevelParameterfv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetTexLevelParameteriv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetTexLevelParameteriv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetTexLevelParameteriv = FnPtr::new(metaloadfn(&mut loadfn, "glGetTexLevelParameteriv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetTexParameterIiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetTexParameterIiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetTexParameterIiv = FnPtr::new(metaloadfn(&mut loadfn, "glGetTexParameterIiv", &["glGetTexParameterIivEXT", "glGetTexParameterIivOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetTexParameterIuiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetTexParameterIuiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetTexParameterIuiv = FnPtr::new(metaloadfn(&mut loadfn, "glGetTexParameterIuiv", &["glGetTexParameterIuivEXT", "glGetTexParameterIuivOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetTexParameterfv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetTexParameterfv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetTexParameterfv = FnPtr::new(metaloadfn(&mut loadfn, "glGetTexParameterfv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetTexParameteriv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetTexParameteriv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetTexParameteriv = FnPtr::new(metaloadfn(&mut loadfn, "glGetTexParameteriv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetTransformFeedbackVarying {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetTransformFeedbackVarying.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetTransformFeedbackVarying = FnPtr::new(metaloadfn(&mut loadfn, "glGetTransformFeedbackVarying", &["glGetTransformFeedbackVaryingEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetUniformBlockIndex {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetUniformBlockIndex.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetUniformBlockIndex = FnPtr::new(metaloadfn(&mut loadfn, "glGetUniformBlockIndex", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetUniformIndices {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetUniformIndices.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetUniformIndices = FnPtr::new(metaloadfn(&mut loadfn, "glGetUniformIndices", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetUniformLocation {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetUniformLocation.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetUniformLocation = FnPtr::new(metaloadfn(&mut loadfn, "glGetUniformLocation", &["glGetUniformLocationARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetUniformfv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetUniformfv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetUniformfv = FnPtr::new(metaloadfn(&mut loadfn, "glGetUniformfv", &["glGetUniformfvARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetUniformiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetUniformiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetUniformiv = FnPtr::new(metaloadfn(&mut loadfn, "glGetUniformiv", &["glGetUniformivARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetUniformuiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetUniformuiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetUniformuiv = FnPtr::new(metaloadfn(&mut loadfn, "glGetUniformuiv", &["glGetUniformuivEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetVertexAttribIiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetVertexAttribIiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetVertexAttribIiv = FnPtr::new(metaloadfn(&mut loadfn, "glGetVertexAttribIiv", &["glGetVertexAttribIivEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetVertexAttribIuiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetVertexAttribIuiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetVertexAttribIuiv = FnPtr::new(metaloadfn(&mut loadfn, "glGetVertexAttribIuiv", &["glGetVertexAttribIuivEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetVertexAttribPointerv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetVertexAttribPointerv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetVertexAttribPointerv = FnPtr::new(metaloadfn(&mut loadfn, "glGetVertexAttribPointerv", &["glGetVertexAttribPointervARB", "glGetVertexAttribPointervNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetVertexAttribdv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetVertexAttribdv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetVertexAttribdv = FnPtr::new(metaloadfn(&mut loadfn, "glGetVertexAttribdv", &["glGetVertexAttribdvARB", "glGetVertexAttribdvNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetVertexAttribfv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetVertexAttribfv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetVertexAttribfv = FnPtr::new(metaloadfn(&mut loadfn, "glGetVertexAttribfv", &["glGetVertexAttribfvARB", "glGetVertexAttribfvNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod GetVertexAttribiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::GetVertexAttribiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::GetVertexAttribiv = FnPtr::new(metaloadfn(&mut loadfn, "glGetVertexAttribiv", &["glGetVertexAttribivARB", "glGetVertexAttribivNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Hint {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Hint.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Hint = FnPtr::new(metaloadfn(&mut loadfn, "glHint", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod IsBuffer {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::IsBuffer.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::IsBuffer = FnPtr::new(metaloadfn(&mut loadfn, "glIsBuffer", &["glIsBufferARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod IsEnabled {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::IsEnabled.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::IsEnabled = FnPtr::new(metaloadfn(&mut loadfn, "glIsEnabled", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod IsEnabledi {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::IsEnabledi.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::IsEnabledi = FnPtr::new(metaloadfn(&mut loadfn, "glIsEnabledi", &["glIsEnabledIndexedEXT", "glIsEnablediEXT", "glIsEnablediNV", "glIsEnablediOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod IsFramebuffer {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::IsFramebuffer.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::IsFramebuffer = FnPtr::new(metaloadfn(&mut loadfn, "glIsFramebuffer", &["glIsFramebufferEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod IsProgram {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::IsProgram.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::IsProgram = FnPtr::new(metaloadfn(&mut loadfn, "glIsProgram", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod IsQuery {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::IsQuery.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::IsQuery = FnPtr::new(metaloadfn(&mut loadfn, "glIsQuery", &["glIsQueryARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod IsRenderbuffer {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::IsRenderbuffer.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::IsRenderbuffer = FnPtr::new(metaloadfn(&mut loadfn, "glIsRenderbuffer", &["glIsRenderbufferEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod IsSampler {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::IsSampler.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::IsSampler = FnPtr::new(metaloadfn(&mut loadfn, "glIsSampler", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod IsShader {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::IsShader.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::IsShader = FnPtr::new(metaloadfn(&mut loadfn, "glIsShader", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod IsSync {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::IsSync.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::IsSync = FnPtr::new(metaloadfn(&mut loadfn, "glIsSync", &["glIsSyncAPPLE"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod IsTexture {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::IsTexture.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::IsTexture = FnPtr::new(metaloadfn(&mut loadfn, "glIsTexture", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod IsVertexArray {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::IsVertexArray.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::IsVertexArray = FnPtr::new(metaloadfn(&mut loadfn, "glIsVertexArray", &["glIsVertexArrayAPPLE", "glIsVertexArrayOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod LineWidth {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::LineWidth.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::LineWidth = FnPtr::new(metaloadfn(&mut loadfn, "glLineWidth", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod LinkProgram {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::LinkProgram.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::LinkProgram = FnPtr::new(metaloadfn(&mut loadfn, "glLinkProgram", &["glLinkProgramARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod LogicOp {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::LogicOp.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::LogicOp = FnPtr::new(metaloadfn(&mut loadfn, "glLogicOp", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod MapBuffer {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::MapBuffer.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::MapBuffer = FnPtr::new(metaloadfn(&mut loadfn, "glMapBuffer", &["glMapBufferARB", "glMapBufferOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod MapBufferRange {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::MapBufferRange.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::MapBufferRange = FnPtr::new(metaloadfn(&mut loadfn, "glMapBufferRange", &["glMapBufferRangeEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod MultiDrawArrays {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::MultiDrawArrays.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::MultiDrawArrays = FnPtr::new(metaloadfn(&mut loadfn, "glMultiDrawArrays", &["glMultiDrawArraysEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod MultiDrawElements {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::MultiDrawElements.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::MultiDrawElements = FnPtr::new(metaloadfn(&mut loadfn, "glMultiDrawElements", &["glMultiDrawElementsEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod MultiDrawElementsBaseVertex {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::MultiDrawElementsBaseVertex.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::MultiDrawElementsBaseVertex = FnPtr::new(metaloadfn(&mut loadfn, "glMultiDrawElementsBaseVertex", &["glMultiDrawElementsBaseVertexEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod MultiTexCoordP1ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::MultiTexCoordP1ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::MultiTexCoordP1ui = FnPtr::new(metaloadfn(&mut loadfn, "glMultiTexCoordP1ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod MultiTexCoordP1uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::MultiTexCoordP1uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::MultiTexCoordP1uiv = FnPtr::new(metaloadfn(&mut loadfn, "glMultiTexCoordP1uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod MultiTexCoordP2ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::MultiTexCoordP2ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::MultiTexCoordP2ui = FnPtr::new(metaloadfn(&mut loadfn, "glMultiTexCoordP2ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod MultiTexCoordP2uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::MultiTexCoordP2uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::MultiTexCoordP2uiv = FnPtr::new(metaloadfn(&mut loadfn, "glMultiTexCoordP2uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod MultiTexCoordP3ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::MultiTexCoordP3ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::MultiTexCoordP3ui = FnPtr::new(metaloadfn(&mut loadfn, "glMultiTexCoordP3ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod MultiTexCoordP3uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::MultiTexCoordP3uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::MultiTexCoordP3uiv = FnPtr::new(metaloadfn(&mut loadfn, "glMultiTexCoordP3uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod MultiTexCoordP4ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::MultiTexCoordP4ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::MultiTexCoordP4ui = FnPtr::new(metaloadfn(&mut loadfn, "glMultiTexCoordP4ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod MultiTexCoordP4uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::MultiTexCoordP4uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::MultiTexCoordP4uiv = FnPtr::new(metaloadfn(&mut loadfn, "glMultiTexCoordP4uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod NormalP3ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::NormalP3ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::NormalP3ui = FnPtr::new(metaloadfn(&mut loadfn, "glNormalP3ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod NormalP3uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::NormalP3uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::NormalP3uiv = FnPtr::new(metaloadfn(&mut loadfn, "glNormalP3uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod PixelStoref {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::PixelStoref.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::PixelStoref = FnPtr::new(metaloadfn(&mut loadfn, "glPixelStoref", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod PixelStorei {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::PixelStorei.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::PixelStorei = FnPtr::new(metaloadfn(&mut loadfn, "glPixelStorei", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod PointParameterf {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::PointParameterf.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::PointParameterf = FnPtr::new(metaloadfn(&mut loadfn, "glPointParameterf", &["glPointParameterfARB", "glPointParameterfEXT", "glPointParameterfSGIS"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod PointParameterfv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::PointParameterfv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::PointParameterfv = FnPtr::new(metaloadfn(&mut loadfn, "glPointParameterfv", &["glPointParameterfvARB", "glPointParameterfvEXT", "glPointParameterfvSGIS"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod PointParameteri {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::PointParameteri.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::PointParameteri = FnPtr::new(metaloadfn(&mut loadfn, "glPointParameteri", &["glPointParameteriNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod PointParameteriv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::PointParameteriv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::PointParameteriv = FnPtr::new(metaloadfn(&mut loadfn, "glPointParameteriv", &["glPointParameterivNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod PointSize {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::PointSize.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::PointSize = FnPtr::new(metaloadfn(&mut loadfn, "glPointSize", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod PolygonMode {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::PolygonMode.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::PolygonMode = FnPtr::new(metaloadfn(&mut loadfn, "glPolygonMode", &["glPolygonModeNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod PolygonOffset {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::PolygonOffset.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::PolygonOffset = FnPtr::new(metaloadfn(&mut loadfn, "glPolygonOffset", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod PrimitiveRestartIndex {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::PrimitiveRestartIndex.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::PrimitiveRestartIndex = FnPtr::new(metaloadfn(&mut loadfn, "glPrimitiveRestartIndex", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ProvokingVertex {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ProvokingVertex.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ProvokingVertex = FnPtr::new(metaloadfn(&mut loadfn, "glProvokingVertex", &["glProvokingVertexEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod QueryCounter {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::QueryCounter.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::QueryCounter = FnPtr::new(metaloadfn(&mut loadfn, "glQueryCounter", &["glQueryCounterEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ReadBuffer {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ReadBuffer.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ReadBuffer = FnPtr::new(metaloadfn(&mut loadfn, "glReadBuffer", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ReadPixels {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ReadPixels.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ReadPixels = FnPtr::new(metaloadfn(&mut loadfn, "glReadPixels", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod RenderbufferStorage {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::RenderbufferStorage.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::RenderbufferStorage = FnPtr::new(metaloadfn(&mut loadfn, "glRenderbufferStorage", &["glRenderbufferStorageEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod RenderbufferStorageMultisample {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::RenderbufferStorageMultisample.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::RenderbufferStorageMultisample = FnPtr::new(metaloadfn(&mut loadfn, "glRenderbufferStorageMultisample", &["glRenderbufferStorageMultisampleEXT", "glRenderbufferStorageMultisampleNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod SampleCoverage {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::SampleCoverage.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::SampleCoverage = FnPtr::new(metaloadfn(&mut loadfn, "glSampleCoverage", &["glSampleCoverageARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod SampleMaski {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::SampleMaski.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::SampleMaski = FnPtr::new(metaloadfn(&mut loadfn, "glSampleMaski", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod SamplerParameterIiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::SamplerParameterIiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::SamplerParameterIiv = FnPtr::new(metaloadfn(&mut loadfn, "glSamplerParameterIiv", &["glSamplerParameterIivEXT", "glSamplerParameterIivOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod SamplerParameterIuiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::SamplerParameterIuiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::SamplerParameterIuiv = FnPtr::new(metaloadfn(&mut loadfn, "glSamplerParameterIuiv", &["glSamplerParameterIuivEXT", "glSamplerParameterIuivOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod SamplerParameterf {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::SamplerParameterf.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::SamplerParameterf = FnPtr::new(metaloadfn(&mut loadfn, "glSamplerParameterf", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod SamplerParameterfv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::SamplerParameterfv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::SamplerParameterfv = FnPtr::new(metaloadfn(&mut loadfn, "glSamplerParameterfv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod SamplerParameteri {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::SamplerParameteri.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::SamplerParameteri = FnPtr::new(metaloadfn(&mut loadfn, "glSamplerParameteri", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod SamplerParameteriv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::SamplerParameteriv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::SamplerParameteriv = FnPtr::new(metaloadfn(&mut loadfn, "glSamplerParameteriv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Scissor {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Scissor.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Scissor = FnPtr::new(metaloadfn(&mut loadfn, "glScissor", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod SecondaryColorP3ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::SecondaryColorP3ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::SecondaryColorP3ui = FnPtr::new(metaloadfn(&mut loadfn, "glSecondaryColorP3ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod SecondaryColorP3uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::SecondaryColorP3uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::SecondaryColorP3uiv = FnPtr::new(metaloadfn(&mut loadfn, "glSecondaryColorP3uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ShaderSource {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ShaderSource.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ShaderSource = FnPtr::new(metaloadfn(&mut loadfn, "glShaderSource", &["glShaderSourceARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod StencilFunc {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::StencilFunc.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::StencilFunc = FnPtr::new(metaloadfn(&mut loadfn, "glStencilFunc", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod StencilFuncSeparate {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::StencilFuncSeparate.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::StencilFuncSeparate = FnPtr::new(metaloadfn(&mut loadfn, "glStencilFuncSeparate", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod StencilMask {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::StencilMask.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::StencilMask = FnPtr::new(metaloadfn(&mut loadfn, "glStencilMask", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod StencilMaskSeparate {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::StencilMaskSeparate.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::StencilMaskSeparate = FnPtr::new(metaloadfn(&mut loadfn, "glStencilMaskSeparate", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod StencilOp {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::StencilOp.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::StencilOp = FnPtr::new(metaloadfn(&mut loadfn, "glStencilOp", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod StencilOpSeparate {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::StencilOpSeparate.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::StencilOpSeparate = FnPtr::new(metaloadfn(&mut loadfn, "glStencilOpSeparate", &["glStencilOpSeparateATI"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexBuffer {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexBuffer.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexBuffer = FnPtr::new(metaloadfn(&mut loadfn, "glTexBuffer", &["glTexBufferARB", "glTexBufferEXT", "glTexBufferOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexCoordP1ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexCoordP1ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexCoordP1ui = FnPtr::new(metaloadfn(&mut loadfn, "glTexCoordP1ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexCoordP1uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexCoordP1uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexCoordP1uiv = FnPtr::new(metaloadfn(&mut loadfn, "glTexCoordP1uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexCoordP2ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexCoordP2ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexCoordP2ui = FnPtr::new(metaloadfn(&mut loadfn, "glTexCoordP2ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexCoordP2uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexCoordP2uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexCoordP2uiv = FnPtr::new(metaloadfn(&mut loadfn, "glTexCoordP2uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexCoordP3ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexCoordP3ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexCoordP3ui = FnPtr::new(metaloadfn(&mut loadfn, "glTexCoordP3ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexCoordP3uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexCoordP3uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexCoordP3uiv = FnPtr::new(metaloadfn(&mut loadfn, "glTexCoordP3uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexCoordP4ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexCoordP4ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexCoordP4ui = FnPtr::new(metaloadfn(&mut loadfn, "glTexCoordP4ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexCoordP4uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexCoordP4uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexCoordP4uiv = FnPtr::new(metaloadfn(&mut loadfn, "glTexCoordP4uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexImage1D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexImage1D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexImage1D = FnPtr::new(metaloadfn(&mut loadfn, "glTexImage1D", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexImage2D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexImage2D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexImage2D = FnPtr::new(metaloadfn(&mut loadfn, "glTexImage2D", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexImage2DMultisample {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexImage2DMultisample.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexImage2DMultisample = FnPtr::new(metaloadfn(&mut loadfn, "glTexImage2DMultisample", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexImage3D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexImage3D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexImage3D = FnPtr::new(metaloadfn(&mut loadfn, "glTexImage3D", &["glTexImage3DEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexImage3DMultisample {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexImage3DMultisample.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexImage3DMultisample = FnPtr::new(metaloadfn(&mut loadfn, "glTexImage3DMultisample", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexParameterIiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexParameterIiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexParameterIiv = FnPtr::new(metaloadfn(&mut loadfn, "glTexParameterIiv", &["glTexParameterIivEXT", "glTexParameterIivOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexParameterIuiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexParameterIuiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexParameterIuiv = FnPtr::new(metaloadfn(&mut loadfn, "glTexParameterIuiv", &["glTexParameterIuivEXT", "glTexParameterIuivOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexParameterf {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexParameterf.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexParameterf = FnPtr::new(metaloadfn(&mut loadfn, "glTexParameterf", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexParameterfv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexParameterfv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexParameterfv = FnPtr::new(metaloadfn(&mut loadfn, "glTexParameterfv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexParameteri {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexParameteri.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexParameteri = FnPtr::new(metaloadfn(&mut loadfn, "glTexParameteri", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexParameteriv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexParameteriv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexParameteriv = FnPtr::new(metaloadfn(&mut loadfn, "glTexParameteriv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexSubImage1D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexSubImage1D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexSubImage1D = FnPtr::new(metaloadfn(&mut loadfn, "glTexSubImage1D", &["glTexSubImage1DEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexSubImage2D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexSubImage2D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexSubImage2D = FnPtr::new(metaloadfn(&mut loadfn, "glTexSubImage2D", &["glTexSubImage2DEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TexSubImage3D {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TexSubImage3D.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TexSubImage3D = FnPtr::new(metaloadfn(&mut loadfn, "glTexSubImage3D", &["glTexSubImage3DEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod TransformFeedbackVaryings {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::TransformFeedbackVaryings.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::TransformFeedbackVaryings = FnPtr::new(metaloadfn(&mut loadfn, "glTransformFeedbackVaryings", &["glTransformFeedbackVaryingsEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform1f {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform1f.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform1f = FnPtr::new(metaloadfn(&mut loadfn, "glUniform1f", &["glUniform1fARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform1fv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform1fv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform1fv = FnPtr::new(metaloadfn(&mut loadfn, "glUniform1fv", &["glUniform1fvARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform1i {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform1i.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform1i = FnPtr::new(metaloadfn(&mut loadfn, "glUniform1i", &["glUniform1iARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform1iv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform1iv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform1iv = FnPtr::new(metaloadfn(&mut loadfn, "glUniform1iv", &["glUniform1ivARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform1ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform1ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform1ui = FnPtr::new(metaloadfn(&mut loadfn, "glUniform1ui", &["glUniform1uiEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform1uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform1uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform1uiv = FnPtr::new(metaloadfn(&mut loadfn, "glUniform1uiv", &["glUniform1uivEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform2f {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform2f.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform2f = FnPtr::new(metaloadfn(&mut loadfn, "glUniform2f", &["glUniform2fARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform2fv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform2fv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform2fv = FnPtr::new(metaloadfn(&mut loadfn, "glUniform2fv", &["glUniform2fvARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform2i {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform2i.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform2i = FnPtr::new(metaloadfn(&mut loadfn, "glUniform2i", &["glUniform2iARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform2iv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform2iv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform2iv = FnPtr::new(metaloadfn(&mut loadfn, "glUniform2iv", &["glUniform2ivARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform2ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform2ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform2ui = FnPtr::new(metaloadfn(&mut loadfn, "glUniform2ui", &["glUniform2uiEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform2uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform2uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform2uiv = FnPtr::new(metaloadfn(&mut loadfn, "glUniform2uiv", &["glUniform2uivEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform3f {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform3f.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform3f = FnPtr::new(metaloadfn(&mut loadfn, "glUniform3f", &["glUniform3fARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform3fv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform3fv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform3fv = FnPtr::new(metaloadfn(&mut loadfn, "glUniform3fv", &["glUniform3fvARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform3i {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform3i.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform3i = FnPtr::new(metaloadfn(&mut loadfn, "glUniform3i", &["glUniform3iARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform3iv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform3iv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform3iv = FnPtr::new(metaloadfn(&mut loadfn, "glUniform3iv", &["glUniform3ivARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform3ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform3ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform3ui = FnPtr::new(metaloadfn(&mut loadfn, "glUniform3ui", &["glUniform3uiEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform3uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform3uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform3uiv = FnPtr::new(metaloadfn(&mut loadfn, "glUniform3uiv", &["glUniform3uivEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform4f {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform4f.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform4f = FnPtr::new(metaloadfn(&mut loadfn, "glUniform4f", &["glUniform4fARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform4fv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform4fv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform4fv = FnPtr::new(metaloadfn(&mut loadfn, "glUniform4fv", &["glUniform4fvARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform4i {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform4i.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform4i = FnPtr::new(metaloadfn(&mut loadfn, "glUniform4i", &["glUniform4iARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform4iv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform4iv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform4iv = FnPtr::new(metaloadfn(&mut loadfn, "glUniform4iv", &["glUniform4ivARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform4ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform4ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform4ui = FnPtr::new(metaloadfn(&mut loadfn, "glUniform4ui", &["glUniform4uiEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Uniform4uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Uniform4uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Uniform4uiv = FnPtr::new(metaloadfn(&mut loadfn, "glUniform4uiv", &["glUniform4uivEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod UniformBlockBinding {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::UniformBlockBinding.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::UniformBlockBinding = FnPtr::new(metaloadfn(&mut loadfn, "glUniformBlockBinding", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod UniformMatrix2fv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::UniformMatrix2fv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::UniformMatrix2fv = FnPtr::new(metaloadfn(&mut loadfn, "glUniformMatrix2fv", &["glUniformMatrix2fvARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod UniformMatrix2x3fv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::UniformMatrix2x3fv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::UniformMatrix2x3fv = FnPtr::new(metaloadfn(&mut loadfn, "glUniformMatrix2x3fv", &["glUniformMatrix2x3fvNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod UniformMatrix2x4fv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::UniformMatrix2x4fv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::UniformMatrix2x4fv = FnPtr::new(metaloadfn(&mut loadfn, "glUniformMatrix2x4fv", &["glUniformMatrix2x4fvNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod UniformMatrix3fv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::UniformMatrix3fv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::UniformMatrix3fv = FnPtr::new(metaloadfn(&mut loadfn, "glUniformMatrix3fv", &["glUniformMatrix3fvARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod UniformMatrix3x2fv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::UniformMatrix3x2fv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::UniformMatrix3x2fv = FnPtr::new(metaloadfn(&mut loadfn, "glUniformMatrix3x2fv", &["glUniformMatrix3x2fvNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod UniformMatrix3x4fv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::UniformMatrix3x4fv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::UniformMatrix3x4fv = FnPtr::new(metaloadfn(&mut loadfn, "glUniformMatrix3x4fv", &["glUniformMatrix3x4fvNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod UniformMatrix4fv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::UniformMatrix4fv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::UniformMatrix4fv = FnPtr::new(metaloadfn(&mut loadfn, "glUniformMatrix4fv", &["glUniformMatrix4fvARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod UniformMatrix4x2fv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::UniformMatrix4x2fv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::UniformMatrix4x2fv = FnPtr::new(metaloadfn(&mut loadfn, "glUniformMatrix4x2fv", &["glUniformMatrix4x2fvNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod UniformMatrix4x3fv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::UniformMatrix4x3fv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::UniformMatrix4x3fv = FnPtr::new(metaloadfn(&mut loadfn, "glUniformMatrix4x3fv", &["glUniformMatrix4x3fvNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod UnmapBuffer {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::UnmapBuffer.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::UnmapBuffer = FnPtr::new(metaloadfn(&mut loadfn, "glUnmapBuffer", &["glUnmapBufferARB", "glUnmapBufferOES"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod UseProgram {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::UseProgram.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::UseProgram = FnPtr::new(metaloadfn(&mut loadfn, "glUseProgram", &["glUseProgramObjectARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod ValidateProgram {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::ValidateProgram.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::ValidateProgram = FnPtr::new(metaloadfn(&mut loadfn, "glValidateProgram", &["glValidateProgramARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib1d {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib1d.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib1d = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib1d", &["glVertexAttrib1dARB", "glVertexAttrib1dNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib1dv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib1dv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib1dv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib1dv", &["glVertexAttrib1dvARB", "glVertexAttrib1dvNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib1f {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib1f.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib1f = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib1f", &["glVertexAttrib1fARB", "glVertexAttrib1fNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib1fv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib1fv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib1fv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib1fv", &["glVertexAttrib1fvARB", "glVertexAttrib1fvNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib1s {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib1s.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib1s = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib1s", &["glVertexAttrib1sARB", "glVertexAttrib1sNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib1sv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib1sv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib1sv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib1sv", &["glVertexAttrib1svARB", "glVertexAttrib1svNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib2d {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib2d.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib2d = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib2d", &["glVertexAttrib2dARB", "glVertexAttrib2dNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib2dv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib2dv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib2dv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib2dv", &["glVertexAttrib2dvARB", "glVertexAttrib2dvNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib2f {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib2f.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib2f = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib2f", &["glVertexAttrib2fARB", "glVertexAttrib2fNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib2fv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib2fv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib2fv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib2fv", &["glVertexAttrib2fvARB", "glVertexAttrib2fvNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib2s {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib2s.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib2s = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib2s", &["glVertexAttrib2sARB", "glVertexAttrib2sNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib2sv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib2sv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib2sv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib2sv", &["glVertexAttrib2svARB", "glVertexAttrib2svNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib3d {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib3d.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib3d = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib3d", &["glVertexAttrib3dARB", "glVertexAttrib3dNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib3dv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib3dv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib3dv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib3dv", &["glVertexAttrib3dvARB", "glVertexAttrib3dvNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib3f {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib3f.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib3f = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib3f", &["glVertexAttrib3fARB", "glVertexAttrib3fNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib3fv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib3fv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib3fv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib3fv", &["glVertexAttrib3fvARB", "glVertexAttrib3fvNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib3s {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib3s.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib3s = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib3s", &["glVertexAttrib3sARB", "glVertexAttrib3sNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib3sv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib3sv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib3sv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib3sv", &["glVertexAttrib3svARB", "glVertexAttrib3svNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib4Nbv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib4Nbv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib4Nbv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib4Nbv", &["glVertexAttrib4NbvARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib4Niv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib4Niv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib4Niv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib4Niv", &["glVertexAttrib4NivARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib4Nsv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib4Nsv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib4Nsv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib4Nsv", &["glVertexAttrib4NsvARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib4Nub {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib4Nub.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib4Nub = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib4Nub", &["glVertexAttrib4NubARB", "glVertexAttrib4ubNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib4Nubv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib4Nubv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib4Nubv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib4Nubv", &["glVertexAttrib4NubvARB", "glVertexAttrib4ubvNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib4Nuiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib4Nuiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib4Nuiv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib4Nuiv", &["glVertexAttrib4NuivARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib4Nusv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib4Nusv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib4Nusv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib4Nusv", &["glVertexAttrib4NusvARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib4bv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib4bv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib4bv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib4bv", &["glVertexAttrib4bvARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib4d {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib4d.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib4d = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib4d", &["glVertexAttrib4dARB", "glVertexAttrib4dNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib4dv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib4dv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib4dv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib4dv", &["glVertexAttrib4dvARB", "glVertexAttrib4dvNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib4f {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib4f.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib4f = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib4f", &["glVertexAttrib4fARB", "glVertexAttrib4fNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib4fv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib4fv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib4fv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib4fv", &["glVertexAttrib4fvARB", "glVertexAttrib4fvNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib4iv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib4iv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib4iv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib4iv", &["glVertexAttrib4ivARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib4s {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib4s.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib4s = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib4s", &["glVertexAttrib4sARB", "glVertexAttrib4sNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib4sv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib4sv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib4sv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib4sv", &["glVertexAttrib4svARB", "glVertexAttrib4svNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib4ubv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib4ubv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib4ubv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib4ubv", &["glVertexAttrib4ubvARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib4uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib4uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib4uiv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib4uiv", &["glVertexAttrib4uivARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttrib4usv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttrib4usv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttrib4usv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttrib4usv", &["glVertexAttrib4usvARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribDivisor {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribDivisor.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribDivisor = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribDivisor", &["glVertexAttribDivisorANGLE", "glVertexAttribDivisorARB", "glVertexAttribDivisorEXT", "glVertexAttribDivisorNV"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI1i {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI1i.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI1i = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI1i", &["glVertexAttribI1iEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI1iv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI1iv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI1iv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI1iv", &["glVertexAttribI1ivEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI1ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI1ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI1ui = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI1ui", &["glVertexAttribI1uiEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI1uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI1uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI1uiv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI1uiv", &["glVertexAttribI1uivEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI2i {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI2i.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI2i = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI2i", &["glVertexAttribI2iEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI2iv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI2iv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI2iv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI2iv", &["glVertexAttribI2ivEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI2ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI2ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI2ui = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI2ui", &["glVertexAttribI2uiEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI2uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI2uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI2uiv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI2uiv", &["glVertexAttribI2uivEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI3i {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI3i.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI3i = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI3i", &["glVertexAttribI3iEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI3iv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI3iv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI3iv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI3iv", &["glVertexAttribI3ivEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI3ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI3ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI3ui = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI3ui", &["glVertexAttribI3uiEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI3uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI3uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI3uiv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI3uiv", &["glVertexAttribI3uivEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI4bv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI4bv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI4bv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI4bv", &["glVertexAttribI4bvEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI4i {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI4i.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI4i = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI4i", &["glVertexAttribI4iEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI4iv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI4iv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI4iv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI4iv", &["glVertexAttribI4ivEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI4sv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI4sv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI4sv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI4sv", &["glVertexAttribI4svEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI4ubv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI4ubv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI4ubv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI4ubv", &["glVertexAttribI4ubvEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI4ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI4ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI4ui = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI4ui", &["glVertexAttribI4uiEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI4uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI4uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI4uiv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI4uiv", &["glVertexAttribI4uivEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribI4usv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribI4usv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribI4usv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribI4usv", &["glVertexAttribI4usvEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribIPointer {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribIPointer.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribIPointer = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribIPointer", &["glVertexAttribIPointerEXT"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribP1ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribP1ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribP1ui = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribP1ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribP1uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribP1uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribP1uiv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribP1uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribP2ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribP2ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribP2ui = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribP2ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribP2uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribP2uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribP2uiv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribP2uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribP3ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribP3ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribP3ui = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribP3ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribP3uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribP3uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribP3uiv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribP3uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribP4ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribP4ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribP4ui = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribP4ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribP4uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribP4uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribP4uiv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribP4uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexAttribPointer {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexAttribPointer.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexAttribPointer = FnPtr::new(metaloadfn(&mut loadfn, "glVertexAttribPointer", &["glVertexAttribPointerARB"]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexP2ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexP2ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexP2ui = FnPtr::new(metaloadfn(&mut loadfn, "glVertexP2ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexP2uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexP2uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexP2uiv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexP2uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexP3ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexP3ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexP3ui = FnPtr::new(metaloadfn(&mut loadfn, "glVertexP3ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexP3uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexP3uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexP3uiv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexP3uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexP4ui {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexP4ui.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexP4ui = FnPtr::new(metaloadfn(&mut loadfn, "glVertexP4ui", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod VertexP4uiv {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::VertexP4uiv.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::VertexP4uiv = FnPtr::new(metaloadfn(&mut loadfn, "glVertexP4uiv", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod Viewport {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::Viewport.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::Viewport = FnPtr::new(metaloadfn(&mut loadfn, "glViewport", &[]))
                    }
                }
            }
        

            #[allow(non_snake_case)]
            pub mod WaitSync {
                use super::{storage, metaloadfn};
                use super::__gl_imports::raw;
                use super::FnPtr;

                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {
                    unsafe { storage::WaitSync.is_loaded }
                }

                #[allow(dead_code)]
                pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const raw::c_void {
                    unsafe {
                        storage::WaitSync = FnPtr::new(metaloadfn(&mut loadfn, "glWaitSync", &["glWaitSyncAPPLE"]))
                    }
                }
            }
        
#[inline(never)]
        fn missing_fn_panic() -> ! {
            panic!("gl function was not loaded")
        }
        

        /// Load each OpenGL symbol using a custom load function. This allows for the
        /// use of functions like `glfwGetProcAddress` or `SDL_GL_GetProcAddress`.
        /// ~~~ignore
        /// gl::load_with(|s| glfw.get_proc_address(s));
        /// ~~~
        #[allow(dead_code)]
        pub fn load_with<F>(mut loadfn: F) where F: FnMut(&'static str) -> *const __gl_imports::raw::c_void {
            #[inline(never)]
            fn inner(loadfn: &mut dyn FnMut(&'static str) -> *const __gl_imports::raw::c_void) {
    
ActiveTexture::load_with(&mut *loadfn);
AttachShader::load_with(&mut *loadfn);
BeginConditionalRender::load_with(&mut *loadfn);
BeginQuery::load_with(&mut *loadfn);
BeginTransformFeedback::load_with(&mut *loadfn);
BindAttribLocation::load_with(&mut *loadfn);
BindBuffer::load_with(&mut *loadfn);
BindBufferBase::load_with(&mut *loadfn);
BindBufferRange::load_with(&mut *loadfn);
BindFragDataLocation::load_with(&mut *loadfn);
BindFragDataLocationIndexed::load_with(&mut *loadfn);
BindFramebuffer::load_with(&mut *loadfn);
BindRenderbuffer::load_with(&mut *loadfn);
BindSampler::load_with(&mut *loadfn);
BindTexture::load_with(&mut *loadfn);
BindVertexArray::load_with(&mut *loadfn);
BlendColor::load_with(&mut *loadfn);
BlendEquation::load_with(&mut *loadfn);
BlendEquationSeparate::load_with(&mut *loadfn);
BlendFunc::load_with(&mut *loadfn);
BlendFuncSeparate::load_with(&mut *loadfn);
BlitFramebuffer::load_with(&mut *loadfn);
BufferData::load_with(&mut *loadfn);
BufferSubData::load_with(&mut *loadfn);
CheckFramebufferStatus::load_with(&mut *loadfn);
ClampColor::load_with(&mut *loadfn);
Clear::load_with(&mut *loadfn);
ClearBufferfi::load_with(&mut *loadfn);
ClearBufferfv::load_with(&mut *loadfn);
ClearBufferiv::load_with(&mut *loadfn);
ClearBufferuiv::load_with(&mut *loadfn);
ClearColor::load_with(&mut *loadfn);
ClearDepth::load_with(&mut *loadfn);
ClearStencil::load_with(&mut *loadfn);
ClientWaitSync::load_with(&mut *loadfn);
ColorMask::load_with(&mut *loadfn);
ColorMaski::load_with(&mut *loadfn);
ColorP3ui::load_with(&mut *loadfn);
ColorP3uiv::load_with(&mut *loadfn);
ColorP4ui::load_with(&mut *loadfn);
ColorP4uiv::load_with(&mut *loadfn);
CompileShader::load_with(&mut *loadfn);
CompressedTexImage1D::load_with(&mut *loadfn);
CompressedTexImage2D::load_with(&mut *loadfn);
CompressedTexImage3D::load_with(&mut *loadfn);
CompressedTexSubImage1D::load_with(&mut *loadfn);
CompressedTexSubImage2D::load_with(&mut *loadfn);
CompressedTexSubImage3D::load_with(&mut *loadfn);
CopyBufferSubData::load_with(&mut *loadfn);
CopyTexImage1D::load_with(&mut *loadfn);
CopyTexImage2D::load_with(&mut *loadfn);
CopyTexSubImage1D::load_with(&mut *loadfn);
CopyTexSubImage2D::load_with(&mut *loadfn);
CopyTexSubImage3D::load_with(&mut *loadfn);
CreateProgram::load_with(&mut *loadfn);
CreateShader::load_with(&mut *loadfn);
CullFace::load_with(&mut *loadfn);
DeleteBuffers::load_with(&mut *loadfn);
DeleteFramebuffers::load_with(&mut *loadfn);
DeleteProgram::load_with(&mut *loadfn);
DeleteQueries::load_with(&mut *loadfn);
DeleteRenderbuffers::load_with(&mut *loadfn);
DeleteSamplers::load_with(&mut *loadfn);
DeleteShader::load_with(&mut *loadfn);
DeleteSync::load_with(&mut *loadfn);
DeleteTextures::load_with(&mut *loadfn);
DeleteVertexArrays::load_with(&mut *loadfn);
DepthFunc::load_with(&mut *loadfn);
DepthMask::load_with(&mut *loadfn);
DepthRange::load_with(&mut *loadfn);
DetachShader::load_with(&mut *loadfn);
Disable::load_with(&mut *loadfn);
DisableVertexAttribArray::load_with(&mut *loadfn);
Disablei::load_with(&mut *loadfn);
DrawArrays::load_with(&mut *loadfn);
DrawArraysInstanced::load_with(&mut *loadfn);
DrawBuffer::load_with(&mut *loadfn);
DrawBuffers::load_with(&mut *loadfn);
DrawElements::load_with(&mut *loadfn);
DrawElementsBaseVertex::load_with(&mut *loadfn);
DrawElementsInstanced::load_with(&mut *loadfn);
DrawElementsInstancedBaseVertex::load_with(&mut *loadfn);
DrawRangeElements::load_with(&mut *loadfn);
DrawRangeElementsBaseVertex::load_with(&mut *loadfn);
Enable::load_with(&mut *loadfn);
EnableVertexAttribArray::load_with(&mut *loadfn);
Enablei::load_with(&mut *loadfn);
EndConditionalRender::load_with(&mut *loadfn);
EndQuery::load_with(&mut *loadfn);
EndTransformFeedback::load_with(&mut *loadfn);
FenceSync::load_with(&mut *loadfn);
Finish::load_with(&mut *loadfn);
Flush::load_with(&mut *loadfn);
FlushMappedBufferRange::load_with(&mut *loadfn);
FramebufferRenderbuffer::load_with(&mut *loadfn);
FramebufferTexture::load_with(&mut *loadfn);
FramebufferTexture1D::load_with(&mut *loadfn);
FramebufferTexture2D::load_with(&mut *loadfn);
FramebufferTexture3D::load_with(&mut *loadfn);
FramebufferTextureLayer::load_with(&mut *loadfn);
FrontFace::load_with(&mut *loadfn);
GenBuffers::load_with(&mut *loadfn);
GenFramebuffers::load_with(&mut *loadfn);
GenQueries::load_with(&mut *loadfn);
GenRenderbuffers::load_with(&mut *loadfn);
GenSamplers::load_with(&mut *loadfn);
GenTextures::load_with(&mut *loadfn);
GenVertexArrays::load_with(&mut *loadfn);
GenerateMipmap::load_with(&mut *loadfn);
GetActiveAttrib::load_with(&mut *loadfn);
GetActiveUniform::load_with(&mut *loadfn);
GetActiveUniformBlockName::load_with(&mut *loadfn);
GetActiveUniformBlockiv::load_with(&mut *loadfn);
GetActiveUniformName::load_with(&mut *loadfn);
GetActiveUniformsiv::load_with(&mut *loadfn);
GetAttachedShaders::load_with(&mut *loadfn);
GetAttribLocation::load_with(&mut *loadfn);
GetBooleani_v::load_with(&mut *loadfn);
GetBooleanv::load_with(&mut *loadfn);
GetBufferParameteri64v::load_with(&mut *loadfn);
GetBufferParameteriv::load_with(&mut *loadfn);
GetBufferPointerv::load_with(&mut *loadfn);
GetBufferSubData::load_with(&mut *loadfn);
GetCompressedTexImage::load_with(&mut *loadfn);
GetDoublev::load_with(&mut *loadfn);
GetError::load_with(&mut *loadfn);
GetFloatv::load_with(&mut *loadfn);
GetFragDataIndex::load_with(&mut *loadfn);
GetFragDataLocation::load_with(&mut *loadfn);
GetFramebufferAttachmentParameteriv::load_with(&mut *loadfn);
GetInteger64i_v::load_with(&mut *loadfn);
GetInteger64v::load_with(&mut *loadfn);
GetIntegeri_v::load_with(&mut *loadfn);
GetIntegerv::load_with(&mut *loadfn);
GetMultisamplefv::load_with(&mut *loadfn);
GetProgramInfoLog::load_with(&mut *loadfn);
GetProgramiv::load_with(&mut *loadfn);
GetQueryObjecti64v::load_with(&mut *loadfn);
GetQueryObjectiv::load_with(&mut *loadfn);
GetQueryObjectui64v::load_with(&mut *loadfn);
GetQueryObjectuiv::load_with(&mut *loadfn);
GetQueryiv::load_with(&mut *loadfn);
GetRenderbufferParameteriv::load_with(&mut *loadfn);
GetSamplerParameterIiv::load_with(&mut *loadfn);
GetSamplerParameterIuiv::load_with(&mut *loadfn);
GetSamplerParameterfv::load_with(&mut *loadfn);
GetSamplerParameteriv::load_with(&mut *loadfn);
GetShaderInfoLog::load_with(&mut *loadfn);
GetShaderSource::load_with(&mut *loadfn);
GetShaderiv::load_with(&mut *loadfn);
GetString::load_with(&mut *loadfn);
GetStringi::load_with(&mut *loadfn);
GetSynciv::load_with(&mut *loadfn);
GetTexImage::load_with(&mut *loadfn);
GetTexLevelParameterfv::load_with(&mut *loadfn);
GetTexLevelParameteriv::load_with(&mut *loadfn);
GetTexParameterIiv::load_with(&mut *loadfn);
GetTexParameterIuiv::load_with(&mut *loadfn);
GetTexParameterfv::load_with(&mut *loadfn);
GetTexParameteriv::load_with(&mut *loadfn);
GetTransformFeedbackVarying::load_with(&mut *loadfn);
GetUniformBlockIndex::load_with(&mut *loadfn);
GetUniformIndices::load_with(&mut *loadfn);
GetUniformLocation::load_with(&mut *loadfn);
GetUniformfv::load_with(&mut *loadfn);
GetUniformiv::load_with(&mut *loadfn);
GetUniformuiv::load_with(&mut *loadfn);
GetVertexAttribIiv::load_with(&mut *loadfn);
GetVertexAttribIuiv::load_with(&mut *loadfn);
GetVertexAttribPointerv::load_with(&mut *loadfn);
GetVertexAttribdv::load_with(&mut *loadfn);
GetVertexAttribfv::load_with(&mut *loadfn);
GetVertexAttribiv::load_with(&mut *loadfn);
Hint::load_with(&mut *loadfn);
IsBuffer::load_with(&mut *loadfn);
IsEnabled::load_with(&mut *loadfn);
IsEnabledi::load_with(&mut *loadfn);
IsFramebuffer::load_with(&mut *loadfn);
IsProgram::load_with(&mut *loadfn);
IsQuery::load_with(&mut *loadfn);
IsRenderbuffer::load_with(&mut *loadfn);
IsSampler::load_with(&mut *loadfn);
IsShader::load_with(&mut *loadfn);
IsSync::load_with(&mut *loadfn);
IsTexture::load_with(&mut *loadfn);
IsVertexArray::load_with(&mut *loadfn);
LineWidth::load_with(&mut *loadfn);
LinkProgram::load_with(&mut *loadfn);
LogicOp::load_with(&mut *loadfn);
MapBuffer::load_with(&mut *loadfn);
MapBufferRange::load_with(&mut *loadfn);
MultiDrawArrays::load_with(&mut *loadfn);
MultiDrawElements::load_with(&mut *loadfn);
MultiDrawElementsBaseVertex::load_with(&mut *loadfn);
MultiTexCoordP1ui::load_with(&mut *loadfn);
MultiTexCoordP1uiv::load_with(&mut *loadfn);
MultiTexCoordP2ui::load_with(&mut *loadfn);
MultiTexCoordP2uiv::load_with(&mut *loadfn);
MultiTexCoordP3ui::load_with(&mut *loadfn);
MultiTexCoordP3uiv::load_with(&mut *loadfn);
MultiTexCoordP4ui::load_with(&mut *loadfn);
MultiTexCoordP4uiv::load_with(&mut *loadfn);
NormalP3ui::load_with(&mut *loadfn);
NormalP3uiv::load_with(&mut *loadfn);
PixelStoref::load_with(&mut *loadfn);
PixelStorei::load_with(&mut *loadfn);
PointParameterf::load_with(&mut *loadfn);
PointParameterfv::load_with(&mut *loadfn);
PointParameteri::load_with(&mut *loadfn);
PointParameteriv::load_with(&mut *loadfn);
PointSize::load_with(&mut *loadfn);
PolygonMode::load_with(&mut *loadfn);
PolygonOffset::load_with(&mut *loadfn);
PrimitiveRestartIndex::load_with(&mut *loadfn);
ProvokingVertex::load_with(&mut *loadfn);
QueryCounter::load_with(&mut *loadfn);
ReadBuffer::load_with(&mut *loadfn);
ReadPixels::load_with(&mut *loadfn);
RenderbufferStorage::load_with(&mut *loadfn);
RenderbufferStorageMultisample::load_with(&mut *loadfn);
SampleCoverage::load_with(&mut *loadfn);
SampleMaski::load_with(&mut *loadfn);
SamplerParameterIiv::load_with(&mut *loadfn);
SamplerParameterIuiv::load_with(&mut *loadfn);
SamplerParameterf::load_with(&mut *loadfn);
SamplerParameterfv::load_with(&mut *loadfn);
SamplerParameteri::load_with(&mut *loadfn);
SamplerParameteriv::load_with(&mut *loadfn);
Scissor::load_with(&mut *loadfn);
SecondaryColorP3ui::load_with(&mut *loadfn);
SecondaryColorP3uiv::load_with(&mut *loadfn);
ShaderSource::load_with(&mut *loadfn);
StencilFunc::load_with(&mut *loadfn);
StencilFuncSeparate::load_with(&mut *loadfn);
StencilMask::load_with(&mut *loadfn);
StencilMaskSeparate::load_with(&mut *loadfn);
StencilOp::load_with(&mut *loadfn);
StencilOpSeparate::load_with(&mut *loadfn);
TexBuffer::load_with(&mut *loadfn);
TexCoordP1ui::load_with(&mut *loadfn);
TexCoordP1uiv::load_with(&mut *loadfn);
TexCoordP2ui::load_with(&mut *loadfn);
TexCoordP2uiv::load_with(&mut *loadfn);
TexCoordP3ui::load_with(&mut *loadfn);
TexCoordP3uiv::load_with(&mut *loadfn);
TexCoordP4ui::load_with(&mut *loadfn);
TexCoordP4uiv::load_with(&mut *loadfn);
TexImage1D::load_with(&mut *loadfn);
TexImage2D::load_with(&mut *loadfn);
TexImage2DMultisample::load_with(&mut *loadfn);
TexImage3D::load_with(&mut *loadfn);
TexImage3DMultisample::load_with(&mut *loadfn);
TexParameterIiv::load_with(&mut *loadfn);
TexParameterIuiv::load_with(&mut *loadfn);
TexParameterf::load_with(&mut *loadfn);
TexParameterfv::load_with(&mut *loadfn);
TexParameteri::load_with(&mut *loadfn);
TexParameteriv::load_with(&mut *loadfn);
TexSubImage1D::load_with(&mut *loadfn);
TexSubImage2D::load_with(&mut *loadfn);
TexSubImage3D::load_with(&mut *loadfn);
TransformFeedbackVaryings::load_with(&mut *loadfn);
Uniform1f::load_with(&mut *loadfn);
Uniform1fv::load_with(&mut *loadfn);
Uniform1i::load_with(&mut *loadfn);
Uniform1iv::load_with(&mut *loadfn);
Uniform1ui::load_with(&mut *loadfn);
Uniform1uiv::load_with(&mut *loadfn);
Uniform2f::load_with(&mut *loadfn);
Uniform2fv::load_with(&mut *loadfn);
Uniform2i::load_with(&mut *loadfn);
Uniform2iv::load_with(&mut *loadfn);
Uniform2ui::load_with(&mut *loadfn);
Uniform2uiv::load_with(&mut *loadfn);
Uniform3f::load_with(&mut *loadfn);
Uniform3fv::load_with(&mut *loadfn);
Uniform3i::load_with(&mut *loadfn);
Uniform3iv::load_with(&mut *loadfn);
Uniform3ui::load_with(&mut *loadfn);
Uniform3uiv::load_with(&mut *loadfn);
Uniform4f::load_with(&mut *loadfn);
Uniform4fv::load_with(&mut *loadfn);
Uniform4i::load_with(&mut *loadfn);
Uniform4iv::load_with(&mut *loadfn);
Uniform4ui::load_with(&mut *loadfn);
Uniform4uiv::load_with(&mut *loadfn);
UniformBlockBinding::load_with(&mut *loadfn);
UniformMatrix2fv::load_with(&mut *loadfn);
UniformMatrix2x3fv::load_with(&mut *loadfn);
UniformMatrix2x4fv::load_with(&mut *loadfn);
UniformMatrix3fv::load_with(&mut *loadfn);
UniformMatrix3x2fv::load_with(&mut *loadfn);
UniformMatrix3x4fv::load_with(&mut *loadfn);
UniformMatrix4fv::load_with(&mut *loadfn);
UniformMatrix4x2fv::load_with(&mut *loadfn);
UniformMatrix4x3fv::load_with(&mut *loadfn);
UnmapBuffer::load_with(&mut *loadfn);
UseProgram::load_with(&mut *loadfn);
ValidateProgram::load_with(&mut *loadfn);
VertexAttrib1d::load_with(&mut *loadfn);
VertexAttrib1dv::load_with(&mut *loadfn);
VertexAttrib1f::load_with(&mut *loadfn);
VertexAttrib1fv::load_with(&mut *loadfn);
VertexAttrib1s::load_with(&mut *loadfn);
VertexAttrib1sv::load_with(&mut *loadfn);
VertexAttrib2d::load_with(&mut *loadfn);
VertexAttrib2dv::load_with(&mut *loadfn);
VertexAttrib2f::load_with(&mut *loadfn);
VertexAttrib2fv::load_with(&mut *loadfn);
VertexAttrib2s::load_with(&mut *loadfn);
VertexAttrib2sv::load_with(&mut *loadfn);
VertexAttrib3d::load_with(&mut *loadfn);
VertexAttrib3dv::load_with(&mut *loadfn);
VertexAttrib3f::load_with(&mut *loadfn);
VertexAttrib3fv::load_with(&mut *loadfn);
VertexAttrib3s::load_with(&mut *loadfn);
VertexAttrib3sv::load_with(&mut *loadfn);
VertexAttrib4Nbv::load_with(&mut *loadfn);
VertexAttrib4Niv::load_with(&mut *loadfn);
VertexAttrib4Nsv::load_with(&mut *loadfn);
VertexAttrib4Nub::load_with(&mut *loadfn);
VertexAttrib4Nubv::load_with(&mut *loadfn);
VertexAttrib4Nuiv::load_with(&mut *loadfn);
VertexAttrib4Nusv::load_with(&mut *loadfn);
VertexAttrib4bv::load_with(&mut *loadfn);
VertexAttrib4d::load_with(&mut *loadfn);
VertexAttrib4dv::load_with(&mut *loadfn);
VertexAttrib4f::load_with(&mut *loadfn);
VertexAttrib4fv::load_with(&mut *loadfn);
VertexAttrib4iv::load_with(&mut *loadfn);
VertexAttrib4s::load_with(&mut *loadfn);
VertexAttrib4sv::load_with(&mut *loadfn);
VertexAttrib4ubv::load_with(&mut *loadfn);
VertexAttrib4uiv::load_with(&mut *loadfn);
VertexAttrib4usv::load_with(&mut *loadfn);
VertexAttribDivisor::load_with(&mut *loadfn);
VertexAttribI1i::load_with(&mut *loadfn);
VertexAttribI1iv::load_with(&mut *loadfn);
VertexAttribI1ui::load_with(&mut *loadfn);
VertexAttribI1uiv::load_with(&mut *loadfn);
VertexAttribI2i::load_with(&mut *loadfn);
VertexAttribI2iv::load_with(&mut *loadfn);
VertexAttribI2ui::load_with(&mut *loadfn);
VertexAttribI2uiv::load_with(&mut *loadfn);
VertexAttribI3i::load_with(&mut *loadfn);
VertexAttribI3iv::load_with(&mut *loadfn);
VertexAttribI3ui::load_with(&mut *loadfn);
VertexAttribI3uiv::load_with(&mut *loadfn);
VertexAttribI4bv::load_with(&mut *loadfn);
VertexAttribI4i::load_with(&mut *loadfn);
VertexAttribI4iv::load_with(&mut *loadfn);
VertexAttribI4sv::load_with(&mut *loadfn);
VertexAttribI4ubv::load_with(&mut *loadfn);
VertexAttribI4ui::load_with(&mut *loadfn);
VertexAttribI4uiv::load_with(&mut *loadfn);
VertexAttribI4usv::load_with(&mut *loadfn);
VertexAttribIPointer::load_with(&mut *loadfn);
VertexAttribP1ui::load_with(&mut *loadfn);
VertexAttribP1uiv::load_with(&mut *loadfn);
VertexAttribP2ui::load_with(&mut *loadfn);
VertexAttribP2uiv::load_with(&mut *loadfn);
VertexAttribP3ui::load_with(&mut *loadfn);
VertexAttribP3uiv::load_with(&mut *loadfn);
VertexAttribP4ui::load_with(&mut *loadfn);
VertexAttribP4uiv::load_with(&mut *loadfn);
VertexAttribPointer::load_with(&mut *loadfn);
VertexP2ui::load_with(&mut *loadfn);
VertexP2uiv::load_with(&mut *loadfn);
VertexP3ui::load_with(&mut *loadfn);
VertexP3uiv::load_with(&mut *loadfn);
VertexP4ui::load_with(&mut *loadfn);
VertexP4uiv::load_with(&mut *loadfn);
Viewport::load_with(&mut *loadfn);
WaitSync::load_with(&mut *loadfn);

            }

            inner(&mut loadfn)
        }
    
