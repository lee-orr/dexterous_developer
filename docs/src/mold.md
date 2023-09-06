# Mold and Alternative Linkers

When running on Linux, and not cross-compiling, you can pass in alternative linkers like mold using the `DEXTEROUS_DEVELOPER_LD_PATH` environment variable. The value then gets inserted as `link-arg=-fuse-ld={DEXTEROUS_DEVELOPER_LD_PATH}` in the rustc compiler arguments.
