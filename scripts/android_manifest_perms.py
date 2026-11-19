"""Doplni do vygenerovaneho AndroidManifest.xml opravneni pro file manager.

Spousti se z scripts/build-android.bat / .sh po `cargo tauri android init`.
Je idempotentni - opakovane spusteni nic nerozbije.
"""

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MANIFEST = os.path.join(
    ROOT, "apps", "filemanager", "src-tauri", "gen", "android",
    "app", "src", "main", "AndroidManifest.xml",
)

PERMISSIONS = [
    ('android.permission.READ_EXTERNAL_STORAGE', ' android:maxSdkVersion="32"'),
    ('android.permission.WRITE_EXTERNAL_STORAGE', ' android:maxSdkVersion="32"'),
    ('android.permission.MANAGE_EXTERNAL_STORAGE', ''),
    ('android.permission.READ_MEDIA_IMAGES', ''),
    ('android.permission.READ_MEDIA_VIDEO', ''),
    ('android.permission.READ_MEDIA_AUDIO', ''),
]


def main() -> int:
    if not os.path.exists(MANIFEST):
        print("CHYBA: chybi %s\nSpust nejdriv: cd apps/filemanager && cargo tauri android init" % MANIFEST)
        return 1

    with open(MANIFEST, "r", encoding="utf-8") as f:
        text = f.read()

    missing = [
        '<uses-permission android:name="%s"%s />' % (name, extra)
        for name, extra in PERMISSIONS
        if 'android:name="%s"' % name not in text
    ]

    if not missing:
        print("AndroidManifest.xml: opravneni uz jsou, nic nemenim.")
        return 0

    block = "\n".join(missing) + "\n"
    if "<application" in text:
        text = text.replace("<application", block + "\n<application", 1)
    else:
        text = re.sub(r"</manifest>\s*$", block + "\n</manifest>\n", text)

    with open(MANIFEST, "w", encoding="utf-8") as f:
        f.write(text)

    print("AndroidManifest.xml: pridano %d opravneni." % len(missing))
    print("Po instalaci jeste na telefonu povol: Nastaveni > Aplikace > File manager in RUST > Pristup ke vsem souborum.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
