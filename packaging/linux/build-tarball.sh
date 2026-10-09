#!/usr/bin/env bash
# Repack the AppImage into a plain tar.gz: the same files, unpacked anywhere and started with no FUSE and no
# mount. Runs INSIDE the image from packaging/linux/Dockerfile, right after build-appimage.sh (see justfile ->
# pkg-linux), and takes the AppImage that script left in /dist.
#
# MADE FROM THE APPIMAGE, NOT BUILT AGAIN. The binary, the libraries and the checks of what is carried and
# what is not live in build-appimage.sh; a second build here would be a second list of libraries to drift
# from the first. Unpacked, the AppImage is already the layout this archive wants - the AUR package
# (packaging/aur/PKGBUILD) relies on the same thing.
#
#   qymcad-<ver>-x86_64-linux/
#     bin/qymcad                    RUNPATH $ORIGIN/../lib, so it finds lib/ wherever the folder is unpacked
#     lib/                          OCCT and the libraries opened by name at run time
#     share/applications/           qymcad.desktop
#     share/icons/hicolor/...       the icons
#     share/doc/qymcad/             LICENSE, THIRD-PARTY-NOTICES.md
set -euo pipefail

# THE NEWEST ONE: a local ./dist keeps the AppImages of earlier builds beside the one just made.
APPIMAGE_FILE=$(ls -t /dist/*-x86_64.AppImage 2>/dev/null | head -1 || true)
if [ -z "$APPIMAGE_FILE" ]; then
    echo "!!! no AppImage in /dist - run build-appimage.sh first"
    exit 1
fi
echo ">>> repacking $APPIMAGE_FILE"

# qymcad-<ver>-x86_64.AppImage -> qymcad-<ver>-x86_64-linux: the name the AppImage got from the tag or the
# commit, so the two packages of one run are named alike.
NAME="$(basename "$APPIMAGE_FILE" .AppImage)-linux"

WORK=/tmp/tarball
rm -rf "$WORK"
mkdir -p "$WORK"
( cd "$WORK" && "$APPIMAGE_FILE" --appimage-extract >/dev/null )
PKG="$WORK/$NAME"
mv "$WORK/squashfs-root/usr" "$PKG"
# `--appimage-extract` writes the directories it makes as 0700 (the AUR package met the same): directories and
# executables become 755, everything else 644, or nobody but the one who unpacked it could start it.
chmod -R u=rwX,go=rX "$PKG"

# --- THE FOLDER IS CHECKED BEFORE IT IS PACKED ---

for f in bin/qymcad share/applications/qymcad.desktop share/doc/qymcad/LICENSE; do
    [ -e "$PKG/$f" ] || { echo "!!! the unpacked AppImage has no $f"; exit 1; }
done

# The binary must look for its libraries beside itself, or it starts only where it was built.
runpath=$(readelf -d "$PKG/bin/qymcad" | awk '/RUNPATH|RPATH/ {print $NF}')
echo ">>> RUNPATH: $runpath"
case "$runpath" in
    *'$ORIGIN/../lib'*) ;;
    *) echo "!!! the binary does not look for its libraries in ../lib: $runpath"; exit 1 ;;
esac

# Every OCCT module must come from the package and not from /opt/occt of this image. RUNPATH is searched
# before the loader's cache; LD_LIBRARY_PATH, which the image sets for the build, would hide a broken one.
outside=$(env -u LD_LIBRARY_PATH ldd "$PKG/bin/qymcad" | awk -v pkg="$PKG/" '/libTK/ && index($3, pkg) != 1 {print}')
if [ -n "$outside" ]; then
    echo "!!! kernel libraries resolved outside the package:"
    printf '%s\n' "$outside"
    exit 1
fi

OUT="/dist/$NAME.tar.gz"
tar -C "$WORK" -czf "$OUT" "$NAME"
rm -rf "$WORK"
echo ">>> DONE: $OUT ($(du -h "$OUT" | cut -f1))"
