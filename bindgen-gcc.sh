
# Rust bindgen uses Clang to generate bindings, but that means that it can't
# find the "system" or compiler headers when the stdenv compiler is GCC.
# This script tells it where to find them.

echo "Extending BINDGEN_EXTRA_CLANG_ARGS with system include paths..." 2>&1
BINDGEN_EXTRA_CLANG_ARGS="${BINDGEN_EXTRA_CLANG_ARGS:-}"
export BINDGEN_EXTRA_CLANG_ARGS
include_paths=$(
  LC_ALL=C $NIX_CC_UNWRAPPED -v -E -x c - </dev/null 2>&1 \
  | awk '/#include <...> search starts here:/{flag=1;next} \
        /End of search list./{flag=0} \
        flag==1 {print $1}'
)
include_args=$(printf '%s\n' "$include_paths" | awk 'NF {printf " -I%s", $1; printf " - %s\n", $1 > "/dev/stderr"}')
BINDGEN_EXTRA_CLANG_ARGS="$BINDGEN_EXTRA_CLANG_ARGS$include_args"
