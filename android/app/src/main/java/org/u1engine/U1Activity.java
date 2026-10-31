// SPDX-License-Identifier: GPL-3.0-or-later
package org.u1engine;

import android.content.Intent;
import java.io.File;
import org.libsdl.app.SDLActivity;

/** The game: SDL's activity running libmain.so (crates/u1-android). */
public class U1Activity extends SDLActivity {
    @Override
    protected String[] getLibraries() {
        return new String[] {"SDL2", "main"};
    }

    @Override
    protected String[] getArguments() {
        return new String[] {"--game-dir", new File(getFilesDir(), "ULTIMA1").getPath()};
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        // "Import again" in the options leaves this marker and quits.
        File marker = new File(getFilesDir(), "reimport");
        if (marker.delete()) {
            Intent i = new Intent(this, ImportActivity.class);
            i.putExtra("reimport", true);
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            startActivity(i);
        }
        // SDL cannot start twice in one process: end it, and the next start
        // gets a fresh one.
        System.exit(0);
    }
}
