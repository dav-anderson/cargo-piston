//NOTE
//this class is compiled into the app's own build-time classes.dex (declared in the
//AndroidManifest.xml), NOT loaded at runtime via InMemoryDexClassLoader like PhotoPickerHelper -
//it must be resolvable by Android's normal ClassLoader at Activity-launch time.

//compile with java
// javac --release 8 -cp /absolute/path/to/android.jar PhotoPickerActivity.java

package com.maverick.photo;

import android.app.Activity;
import android.content.Intent;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.Matrix;
import android.media.ExifInterface;
import android.net.Uri;
import android.os.Bundle;

import java.io.InputStream;

public class PhotoPickerActivity extends Activity {

    private static final int REQUEST_CODE_PICK_IMAGE = 1001;
    private long callbackPtr;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        callbackPtr = getIntent().getLongExtra("callback_ptr", 0);

        Intent intent = new Intent(Intent.ACTION_GET_CONTENT);
        intent.setType("image/*");
        intent.addCategory(Intent.CATEGORY_OPENABLE);

        try {
            startActivityForResult(Intent.createChooser(intent, "Select Picture"), REQUEST_CODE_PICK_IMAGE);
        } catch (Exception e) {
            finishWithResult(null, 0, 0);
        }
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);

        if (requestCode != REQUEST_CODE_PICK_IMAGE || resultCode != RESULT_OK || data == null) {
            finishWithResult(null, 0, 0);
            return;
        }

        try {
            Uri imageUri = data.getData();
            if (imageUri == null) {
                finishWithResult(null, 0, 0);
                return;
            }

            int orientation = ExifInterface.ORIENTATION_NORMAL;
            InputStream exifStream = getContentResolver().openInputStream(imageUri);
            if (exifStream != null) {
                orientation = new ExifInterface(exifStream).getAttributeInt(
                    ExifInterface.TAG_ORIENTATION, ExifInterface.ORIENTATION_NORMAL);
                exifStream.close();
            }

            InputStream inputStream = getContentResolver().openInputStream(imageUri);
            Bitmap bitmap = BitmapFactory.decodeStream(inputStream);
            if (inputStream != null) inputStream.close();

            if (bitmap == null) {
                finishWithResult(null, 0, 0);
                return;
            }

            bitmap = applyExifRotation(bitmap, orientation);

            int width = bitmap.getWidth();
            int height = bitmap.getHeight();
            int[] pixels = new int[width * height];
            bitmap.getPixels(pixels, 0, width, 0, 0, width, height);

            byte[] rgba = new byte[width * height * 4];
            for (int i = 0; i < pixels.length; i++) {
                int pixel = pixels[i];
                rgba[i * 4]     = (byte) ((pixel >> 16) & 0xFF); // R
                rgba[i * 4 + 1] = (byte) ((pixel >> 8)  & 0xFF); // G
                rgba[i * 4 + 2] = (byte) (pixel & 0xFF);         // B
                rgba[i * 4 + 3] = (byte) ((pixel >> 24) & 0xFF); // A
            }

            finishWithResult(rgba, width, height);
        } catch (Exception e) {
            finishWithResult(null, 0, 0);
        }
    }

    private Bitmap applyExifRotation(Bitmap bitmap, int orientation) {
        Matrix matrix = new Matrix();
        switch (orientation) {
            case ExifInterface.ORIENTATION_ROTATE_90:  matrix.postRotate(90);  break;
            case ExifInterface.ORIENTATION_ROTATE_180: matrix.postRotate(180); break;
            case ExifInterface.ORIENTATION_ROTATE_270: matrix.postRotate(270); break;
            case ExifInterface.ORIENTATION_FLIP_HORIZONTAL: matrix.preScale(-1, 1); break;
            case ExifInterface.ORIENTATION_FLIP_VERTICAL:   matrix.preScale(1, -1); break;
            default: return bitmap;
        }
        Bitmap rotated = Bitmap.createBitmap(bitmap, 0, 0, bitmap.getWidth(), bitmap.getHeight(), matrix, true);
        if (rotated != bitmap) {
            bitmap.recycle();
        }
        return rotated;
    }

    private void finishWithResult(byte[] rgbaData, int width, int height) {
        nativeOnPhotoPicked(callbackPtr, rgbaData, width, height);
        finish();
    }

    private static native void nativeOnPhotoPicked(long callbackPtr, byte[] rgbaData, int width, int height);
}
