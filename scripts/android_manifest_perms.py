"""Upravi vygenerovany Android projekt (slozka gen/ se neukládá do gitu):

  1. doplni do AndroidManifest.xml opravneni pro file manager,
  2. upravi MainActivity.kt, aby obsah nelezl pod stavovou listu (hodiny, baterie)
     a pod navigacni listu (tlacitka Zpet / Domu) - Android 15+ vykresluje appky
     "od kraje ke kraji" a bez teto upravy je listy prekryvaji.

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


ACTIVITY = os.path.join(
    ROOT, "apps", "filemanager", "src-tauri", "gen", "android",
    "app", "src", "main", "java", "cz", "tvojefm", "filemanager", "MainActivity.kt",
)

ACTIVITY_SRC = """package cz.tvojefm.filemanager

import android.graphics.Color
import android.os.Bundle
import android.view.View
import androidx.activity.SystemBarStyle
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

// FM-INSETS: upraveno skriptem scripts/android_manifest_perms.py
class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    // Tmava appka -> svetle ikony ve stavove a navigacni liste
    enableEdgeToEdge(
      statusBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
      navigationBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
    )
    super.onCreate(savedInstanceState)

    // Obsah se posune o stavovou listu, navigacni listu, vykroj kamery a klavesnici
    val root = findViewById<View>(android.R.id.content)
    root.setBackgroundColor(0xFF0F1115.toInt())
    ViewCompat.setOnApplyWindowInsetsListener(root) { v, insets ->
      val bars = insets.getInsets(
        WindowInsetsCompat.Type.systemBars() or
          WindowInsetsCompat.Type.displayCutout() or
          WindowInsetsCompat.Type.ime()
      )
      v.setPadding(bars.left, bars.top, bars.right, bars.bottom)
      WindowInsetsCompat.CONSUMED
    }
  }
}
"""


def patch_activity() -> int:
    if not os.path.exists(ACTIVITY):
        print("CHYBA: chybi %s" % ACTIVITY)
        return 1
    with open(ACTIVITY, "r", encoding="utf-8") as f:
        text = f.read()
    if "FM-INSETS" in text:
        print("MainActivity.kt: okraje uz jsou upravene, nic nemenim.")
        return 0
    with open(ACTIVITY, "w", encoding="utf-8", newline="\n") as f:
        f.write(ACTIVITY_SRC)
    print("MainActivity.kt: obsah se nyni drzi mezi stavovou a navigacni listou.")
    return 0


def main() -> int:
    rc = patch_manifest()
    rc2 = patch_activity()
    return rc or rc2


def patch_manifest() -> int:
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
