"""Upravi vygenerovany Android projekt (slozka gen/ se neukládá do gitu):

  1. doplni do AndroidManifest.xml opravneni pro file manager,
  2. upravi MainActivity.kt, aby obsah nelezl pod stavovou listu (hodiny, baterie)
     a pod navigacni listu (tlacitka Zpet / Domu) - Android 15+ vykresluje appky
     "od kraje ke kraji" a bez teto upravy je listy prekryvaji,
  3. nastavi podepisovani release APK klicem z ~/.android-keys/filemanager-release.properties
     (klic ani hesla nejsou v repozitari; viz docs/INSTALACE.md).

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


GRADLE = os.path.join(
    ROOT, "apps", "filemanager", "src-tauri", "gen", "android", "app", "build.gradle.kts",
)

SIGN_BLOCK = """    // FM-SIGNING: klic pro release APK (mimo repozitar), upraveno skriptem
    val fmKeyProps = Properties().apply {
        val f = file(System.getenv("FM_KEYSTORE_PROPERTIES")
            ?: (System.getProperty("user.home") + "/.android-keys/filemanager-release.properties"))
        if (f.exists()) f.inputStream().use { load(it) }
    }
    signingConfigs {
        if (fmKeyProps.containsKey("storeFile")) {
            create("fmRelease") {
                storeFile = file(fmKeyProps.getProperty("storeFile"))
                storePassword = fmKeyProps.getProperty("storePassword")
                keyAlias = fmKeyProps.getProperty("keyAlias")
                keyPassword = fmKeyProps.getProperty("keyPassword")
            }
        }
    }
"""


def patch_signing() -> int:
    if not os.path.exists(GRADLE):
        print("CHYBA: chybi %s" % GRADLE)
        return 1
    with open(GRADLE, "r", encoding="utf-8") as f:
        text = f.read()
    if "FM-SIGNING" in text:
        print("build.gradle.kts: podepisovani uz je nastavene, nic nemenim.")
        return 0
    if "    buildTypes {\n" not in text or '        getByName("release") {\n' not in text:
        print("POZOR: build.gradle.kts ma jinou strukturu, podepisovani nenastaveno.")
        return 1
    text = text.replace("    buildTypes {\n", SIGN_BLOCK + "    buildTypes {\n", 1)
    text = text.replace(
        '        getByName("release") {\n',
        '        getByName("release") {\n'
        '            signingConfigs.findByName("fmRelease")?.let { signingConfig = it }\n', 1)
    with open(GRADLE, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    print("build.gradle.kts: release APK se bude podepisovat klicem z ~/.android-keys.")
    return 0


def main() -> int:
    rc = patch_manifest()
    rc2 = patch_activity()
    rc3 = patch_signing()
    return rc or rc2 or rc3


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
