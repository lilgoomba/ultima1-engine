// SPDX-License-Identifier: GPL-3.0-or-later
package org.u1engine;

import android.app.Activity;
import android.content.ContentResolver;
import android.content.Intent;
import android.database.Cursor;
import android.graphics.Color;
import android.net.Uri;
import android.os.Bundle;
import android.provider.DocumentsContract;
import android.view.Gravity;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.zip.ZipEntry;
import java.util.zip.ZipInputStream;

/**
 * Brings the player's own Ultima I files into the app's private storage:
 * from a folder (the system's folder picker) or a .zip. The files are only
 * read where they are; the copies are checked as the desktop program
 * checks a game folder before the game starts with them. Nothing of the
 * game comes with the app.
 */
public class ImportActivity extends Activity {
    static {
        System.loadLibrary("SDL2");
        System.loadLibrary("main");
    }

    /** "" when `dir` holds everything the game needs, else what it lacks. */
    static native String missingFiles(String dir);

    private static final int PICK_FOLDER = 1, PICK_ZIP = 2;
    /** No game file is near this size: a guard against picking a whole drive. */
    private static final long MAX_TOTAL = 64L << 20;

    private TextView status;
    private Button folder, zip, back;

    private File gameDir() {
        return new File(getFilesDir(), "ULTIMA1");
    }

    @Override
    protected void onCreate(Bundle saved) {
        super.onCreate(saved);
        boolean again = getIntent().getBooleanExtra("reimport", false);
        if (!again && gameDir().isDirectory() && missingFiles(gameDir().getPath()).isEmpty()) {
            play();
            return;
        }
        LinearLayout l = new LinearLayout(this);
        l.setOrientation(LinearLayout.VERTICAL);
        l.setGravity(Gravity.CENTER_HORIZONTAL);
        int pad = (int) (24 * getResources().getDisplayMetrics().density);
        l.setPadding(pad, pad, pad, pad);
        l.setBackgroundColor(Color.BLACK);
        TextView t = new TextView(this);
        t.setTextColor(Color.WHITE);
        t.setTextSize(18);
        t.setText("Ultima I needs your own copy of the game.\n\n"
                + "Choose the folder that holds ULTIMA.EXE, OUT.EXE, MAP.BIN and the rest "
                + "(for example copied from your PC to Download/ULTIMA1), or a .zip of it. "
                + "The files are copied into this app's own storage; the originals are left as they are.");
        l.addView(t);
        folder = new Button(this);
        folder.setText("Choose the game folder");
        folder.setOnClickListener(v -> {
            Intent i = new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE);
            startActivityForResult(i, PICK_FOLDER);
        });
        l.addView(folder);
        zip = new Button(this);
        zip.setText("Choose a .zip");
        zip.setOnClickListener(v -> {
            Intent i = new Intent(Intent.ACTION_OPEN_DOCUMENT);
            i.addCategory(Intent.CATEGORY_OPENABLE);
            i.setType("*/*");
            i.putExtra(Intent.EXTRA_MIME_TYPES, new String[] {"application/zip", "application/x-zip-compressed", "application/octet-stream"});
            startActivityForResult(i, PICK_ZIP);
        });
        l.addView(zip);
        back = new Button(this);
        back.setText("Keep the files I have");
        back.setOnClickListener(v -> play());
        if (gameDir().isDirectory() && missingFiles(gameDir().getPath()).isEmpty()) l.addView(back);
        status = new TextView(this);
        status.setTextColor(Color.YELLOW);
        status.setTextSize(16);
        status.setPadding(0, pad, 0, 0);
        l.addView(status);
        ScrollView s = new ScrollView(this);
        s.setBackgroundColor(Color.BLACK);
        s.addView(l);
        setContentView(s);
    }

    private void play() {
        startActivity(new Intent(this, U1Activity.class));
        finish();
    }

    private void busy(boolean b, String msg) {
        runOnUiThread(() -> {
            folder.setEnabled(!b);
            zip.setEnabled(!b);
            status.setText(msg);
        });
    }

    @Override
    protected void onActivityResult(int req, int res, Intent data) {
        if (res != RESULT_OK || data == null || data.getData() == null) return;
        Uri uri = data.getData();
        busy(true, "Copying…");
        new Thread(() -> {
            File tmp = new File(getFilesDir(), "ULTIMA1.import");
            try {
                deleteAll(tmp);
                tmp.mkdirs();
                long[] total = {0};
                if (req == PICK_FOLDER) {
                    copyTree(uri, DocumentsContract.getTreeDocumentId(uri), tmp, total, 0);
                } else {
                    copyZip(uri, tmp, total);
                }
                String miss = missingFiles(tmp.getPath());
                if (!miss.isEmpty()) {
                    deleteAll(tmp);
                    busy(false, "That is not a complete Ultima I folder. Missing: " + miss);
                    return;
                }
                deleteAll(gameDir());
                if (!tmp.renameTo(gameDir())) throw new IOException("could not move the files into place");
                runOnUiThread(this::play);
            } catch (Exception e) {
                deleteAll(tmp);
                busy(false, "Could not copy the files: " + e.getMessage());
            }
        }).start();
    }

    /** Copies every file under the folder, flattened into `to`. */
    private void copyTree(Uri tree, String docId, File to, long[] total, int depth) throws IOException {
        if (depth > 3) return;
        ContentResolver cr = getContentResolver();
        Uri kids = DocumentsContract.buildChildDocumentsUriUsingTree(tree, docId);
        String[] cols = {DocumentsContract.Document.COLUMN_DOCUMENT_ID, DocumentsContract.Document.COLUMN_DISPLAY_NAME,
            DocumentsContract.Document.COLUMN_MIME_TYPE};
        try (Cursor c = cr.query(kids, cols, null, null, null)) {
            if (c == null) return;
            while (c.moveToNext()) {
                String id = c.getString(0), name = c.getString(1), mime = c.getString(2);
                if (DocumentsContract.Document.MIME_TYPE_DIR.equals(mime)) {
                    copyTree(tree, id, to, total, depth + 1);
                } else {
                    try (InputStream in = cr.openInputStream(DocumentsContract.buildDocumentUriUsingTree(tree, id))) {
                        copy(in, new File(to, safe(name)), total);
                    }
                }
            }
        }
    }

    private void copyZip(Uri uri, File to, long[] total) throws IOException {
        try (InputStream raw = getContentResolver().openInputStream(uri); ZipInputStream z = new ZipInputStream(raw)) {
            ZipEntry e;
            while ((e = z.getNextEntry()) != null) {
                if (!e.isDirectory()) copy(z, new File(to, safe(e.getName())), total);
            }
        }
    }

    /** The file's own name, without any folders (so nothing lands outside). */
    private static String safe(String n) {
        n = n.substring(Math.max(n.lastIndexOf('/'), n.lastIndexOf('\\')) + 1);
        return n.isEmpty() || n.startsWith(".") ? "_" + n : n;
    }

    private static void copy(InputStream in, File f, long[] total) throws IOException {
        // A file met twice (the same name in two folders): the first wins.
        if (f.exists()) return;
        byte[] buf = new byte[65536];
        try (OutputStream out = new FileOutputStream(f)) {
            int n;
            while ((n = in.read(buf)) > 0) {
                total[0] += n;
                if (total[0] > MAX_TOTAL) throw new IOException("that folder is far bigger than the game");
                out.write(buf, 0, n);
            }
        }
    }

    private static void deleteAll(File f) {
        File[] kids = f.listFiles();
        if (kids != null) for (File k : kids) deleteAll(k);
        f.delete();
    }
}
