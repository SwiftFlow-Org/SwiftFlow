#pragma once
#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    float x, y;
} SFScale;

typedef struct {
    float r, g, b, a;
} SFColor;

typedef struct {
    float x, y, width, height;
} SFRect;

typedef struct {
    float top, bottom, leading, trailing;
} SFEdgeInsets;

typedef struct {
    SFColor color;
    float   width;
    float   _pad[3];
} SFBorder;

typedef enum : uint32_t {
    SF_NODE_EMPTY  = 0,
    SF_NODE_RECT   = 1,
    SF_NODE_TEXT   = 2,
    SF_NODE_STACK  = 3,
    SF_NODE_SPACER = 4,
    SF_NODE_IMAGE  = 5,
    SF_NODE_SCROLL = 6,

    SF_NODE_ICON   = 7,
} SFNodeKind;

typedef enum : uint32_t {
    SF_AXIS_VERTICAL   = 0,
    SF_AXIS_HORIZONTAL = 1,
    SF_AXIS_DEPTH      = 2,
} SFAxis;

typedef enum : uint32_t {
    SF_ALIGNMENT_LEADING  = 0,
    SF_ALIGNMENT_CENTER   = 1,
    SF_ALIGNMENT_TRAILING = 2,
} SFAlignment;

typedef enum : uint32_t {
    SF_CONTENT_FIT     = 0,
    SF_CONTENT_FILL    = 1,
    SF_CONTENT_STRETCH = 2,
} SFContentMode;

typedef enum : uint32_t {
    SF_SIZING_HUG   = 0,
    SF_SIZING_FILL  = 1,
    SF_SIZING_FIXED = 2,
} SFSizing;

typedef enum : uint32_t {
    SF_FONT_SANS       = 0,
    SF_FONT_MONOSPACED = 1,
    SF_FONT_SERIF      = 2,
    SF_FONT_ROUNDED    = 3,
    SF_FONT_ICON       = 4,
} SFFontFamily;

typedef struct SFNode {
    uint32_t      node_id;
    SFNodeKind    kind;
    SFRect        frame;
    SFEdgeInsets  padding;

    SFSizing      sizingX;
    SFSizing      sizingY;

    SFScale       scale;

    float         offsetX;
    float         offsetY;

    SFColor       fill;
    SFBorder      border;
    float         cornerRadius;

    float         blurRadius;

    float         specular;

    float         clipContent;

    float         edgeEffectHeight;

    float         shadowRadius;
    float         shadowOpacity;

    float         noMerge;

    float         progressiveBlur;

    float         progressiveStart;

    float         glassRefraction;

    float         glassInteractive;

    const uint8_t* text;
    size_t         textLen;
    float          fontSize;

    float          fontWeight;

    SFFontFamily   fontFamily;

    float          contentBlur;

    uint32_t       lineLimit;

    SFAlignment    textAlign;
    SFColor        color;

    uint32_t       imageId;
    SFContentMode  imageContentMode;

    SFAxis         axis;

    SFAlignment    alignment;
    SFAlignment    verticalAlignment;

    SFAlignment    mainAxisAlignment;
    float          spacing;
    struct SFNode* children;
    size_t         childrenLen;

    float          minLength;

    float          fixedWidth;
    float          fixedHeight;

    uint32_t       scrollId;
    float          contentOffsetX;
    float          contentOffsetY;
    float          contentWidth;
    float          contentHeight;

    float          weight;
} SFNode;

typedef enum : uint32_t {
    SF_SURFACE_METAL_LAYER = 0,
    SF_SURFACE_RAW_HANDLE  = 1,
} SFSurfaceKind;

typedef struct {
    SFSurfaceKind kind;
    void* handle;
    void* display_handle;
} SFSurfaceDescriptor;

void swiftflow_init(SFSurfaceDescriptor surface, uint32_t width, uint32_t height);

void swiftflow_resize(uint32_t width, uint32_t height);

void swiftflow_surface_invalidated(void);

void sf_render_tree(void* root, float width, float height, float scale);

void sf_init(uint32_t width, uint32_t height);

uint32_t sf_hit_test(const SFNode* root, float x, float y);

size_t sf_hit_test_path(const SFNode* root, float x, float y,
                        uint32_t* out, size_t cap);

uint32_t sf_hit_test_scroll(const SFNode* root, float x, float y);

typedef struct {
    uint32_t scrollId;
    uint32_t axis;
} SFScrollHit;

size_t sf_hit_test_scroll_path(const SFNode* root, float x, float y,
                               SFScrollHit* out, size_t cap);

typedef struct {
    float viewportHeight;
    float contentHeight;
    float viewportWidth;
    float contentWidth;
} SFScrollMetrics;

SFScrollMetrics sf_get_scroll_metrics(const SFNode* root, uint32_t scroll_id);

SFRect sf_get_node_frame(const SFNode* root, uint32_t node_id);

typedef struct {
    float width;
    float height;
} SFImageSize;

SFImageSize sf_register_image(uint32_t id, const uint8_t* bytes, size_t len);

void sf_unregister_image(uint32_t id);

SFNode* sf_create_empty_root(void);

void sf_log(const char* message);

void swiftflow_get_atlas_debug(const uint8_t** out_ptr, size_t* out_len);

typedef struct {

    const char* title;

    uint32_t    width;
    uint32_t    height;
} SFDesktopConfig;

typedef struct {
    uint32_t width;
    uint32_t height;
    float    scale;

    float    safeTop;
    float    safeBottom;
    float    safeLeading;
    float    safeTrailing;
} SFDesktopSurfaceInfo;

typedef enum : uint32_t {
    SF_LIFECYCLE_FOREGROUND = 0,
    SF_LIFECYCLE_BACKGROUND = 1,
    SF_LIFECYCLE_TERMINATE  = 2,
} SFLifecycleEvent;

typedef enum : uint32_t {
    SF_KEY_OTHER = 0,
    SF_KEY_BACKSPACE,
    SF_KEY_DELETE,
    SF_KEY_ENTER,
    SF_KEY_TAB,
    SF_KEY_ESCAPE,
    SF_KEY_LEFT,
    SF_KEY_RIGHT,
    SF_KEY_UP,
    SF_KEY_DOWN,
    SF_KEY_HOME,
    SF_KEY_END,
    SF_KEY_PAGE_UP,
    SF_KEY_PAGE_DOWN,
} SFKey;

typedef enum : uint32_t {
    SF_MOD_NONE    = 0,
    SF_MOD_SHIFT   = 1 << 0,
    SF_MOD_CONTROL = 1 << 1,
    SF_MOD_ALT     = 1 << 2,
    SF_MOD_SUPER   = 1 << 3,
} SFKeyModifiers;

typedef struct {

    void (*frame)(float dt);
    void (*pointerDown)(float x, float y, double t);
    void (*pointerMoved)(float x, float y, double t);
    void (*pointerUp)(float x, float y, double t);

    void (*scroll)(float x, float y, float dx, float dy, uint32_t phase);
    void (*resized)(SFDesktopSurfaceInfo info);
    void (*lifecycle)(uint32_t event);

    void (*key)(uint32_t key, uint32_t modifiers, uint32_t pressed, uint32_t isRepeat);

    void (*imePreedit)(const char* text, int32_t cursorBegin, int32_t cursorEnd);

    void (*imeCommit)(const char* text);

    void (*imeEnabled)(uint32_t enabled);
} SFDesktopCallbacks;

void sf_desktop_run(SFDesktopConfig config, SFDesktopCallbacks callbacks);

void sf_desktop_set_ime_allowed(uint32_t allowed);

void sf_desktop_set_ime_cursor_area(float x, float y, float width, float height);

typedef struct {
    uint32_t width;
    uint32_t height;
    float    scale;

    float    safeTop;
    float    safeBottom;
    float    safeLeading;
    float    safeTrailing;

    float    cornerRadius;
} SFAndroidSurfaceInfo;

typedef struct {
    void (*frame)(float dt);
    void (*pointerDown)(float x, float y, double t);
    void (*pointerMoved)(float x, float y, double t);
    void (*pointerUp)(float x, float y, double t);
    void (*resized)(SFAndroidSurfaceInfo info);
    void (*lifecycle)(uint32_t event);

    void (*assetsPath)(const char* path);

    void (*key)(uint32_t key, uint32_t modifiers, uint32_t pressed, uint32_t isRepeat);

    void (*imePreedit)(const char* text, int32_t cursorBegin, int32_t cursorEnd);

    void (*imeCommit)(const char* text);

    void (*imeEnabled)(uint32_t enabled);
} SFAndroidCallbacks;

void sf_android_run(SFAndroidCallbacks callbacks);

void sf_android_set_ime_allowed(uint32_t allowed);

void sf_android_set_ime_cursor_area(float x, float y, float width, float height);

#ifdef __cplusplus
}
#endif
