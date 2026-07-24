package com.hiddenshield.qa.documentsprovider;

import android.database.Cursor;
import android.database.MatrixCursor;
import android.os.CancellationSignal;
import android.os.ParcelFileDescriptor;
import android.provider.DocumentsContract.Document;
import android.provider.DocumentsContract.Root;
import android.provider.DocumentsProvider;

import java.io.FileNotFoundException;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.LinkedHashMap;
import java.util.Map;

public final class RightsEvidenceQaDocumentsProvider extends DocumentsProvider {
    private static final String ROOT_ID = "hiddenshield-r4-root";
    private static final String ROOT_DOCUMENT_ID = "root";
    private static final String CASE_DOCUMENT_ID = "case-fixture-r4-provider";

    private static final String[] ROOT_COLUMNS = {
        Root.COLUMN_ROOT_ID,
        Root.COLUMN_MIME_TYPES,
        Root.COLUMN_FLAGS,
        Root.COLUMN_TITLE,
        Root.COLUMN_SUMMARY,
        Root.COLUMN_DOCUMENT_ID,
        Root.COLUMN_AVAILABLE_BYTES,
    };

    private static final String[] DOCUMENT_COLUMNS = {
        Document.COLUMN_DOCUMENT_ID,
        Document.COLUMN_MIME_TYPE,
        Document.COLUMN_DISPLAY_NAME,
        Document.COLUMN_LAST_MODIFIED,
        Document.COLUMN_FLAGS,
        Document.COLUMN_SIZE,
    };

    private final Map<String, QaDocument> documents = new LinkedHashMap<>();

    @Override
    public boolean onCreate() {
        addDirectory(ROOT_DOCUMENT_ID, null, "HiddenShield QA Provider");
        addDirectory(CASE_DOCUMENT_ID, ROOT_DOCUMENT_ID, "case-fixture-r4-provider");
        addFile(
            "case.json",
            CASE_DOCUMENT_ID,
            "case.json",
            "application/json",
            "case.json"
        );
        addFile(
            "case-manifest.json",
            CASE_DOCUMENT_ID,
            "case-manifest.json",
            "application/json",
            "case-manifest.json"
        );
        addDirectory("attachments", CASE_DOCUMENT_ID, "attachments");
        addDirectory("attachments/original", "attachments", "original");
        addDirectory("attachments/working-copy", "attachments", "working-copy");
        addDirectory("attachments/capture", "attachments", "capture");
        addDirectory(
            "attachments/external-receipt",
            "attachments",
            "external-receipt"
        );
        addFile(
            "attachments/original/ATT-01-original-work.txt",
            "attachments/original",
            "ATT-01-original-work.txt",
            "text/plain",
            "attachments/original/ATT-01-original-work.txt"
        );
        addFile(
            "attachments/working-copy/ATT-02-analysis-copy.txt",
            "attachments/working-copy",
            "ATT-02-analysis-copy.txt",
            "text/plain",
            "attachments/working-copy/ATT-02-analysis-copy.txt"
        );
        addFile(
            "attachments/capture/ATT-03-disputed-page-capture.txt",
            "attachments/capture",
            "ATT-03-disputed-page-capture.txt",
            "text/plain",
            "attachments/capture/ATT-03-disputed-page-capture.txt"
        );
        addFile(
            "attachments/external-receipt/ATT-04-platform-receipt.json",
            "attachments/external-receipt",
            "ATT-04-platform-receipt.json",
            "application/json",
            "attachments/external-receipt/ATT-04-platform-receipt.json"
        );
        return true;
    }

    @Override
    public Cursor queryRoots(String[] projection) {
        MatrixCursor cursor = new MatrixCursor(resolveRootProjection(projection));
        MatrixCursor.RowBuilder row = cursor.newRow();
        row.add(Root.COLUMN_ROOT_ID, ROOT_ID);
        row.add(Root.COLUMN_MIME_TYPES, "*/*");
        row.add(Root.COLUMN_FLAGS, Root.FLAG_SUPPORTS_IS_CHILD | Root.FLAG_LOCAL_ONLY);
        row.add(Root.COLUMN_TITLE, "HiddenShield QA Provider");
        row.add(Root.COLUMN_SUMMARY, "R4 read-only fixture provider");
        row.add(Root.COLUMN_DOCUMENT_ID, ROOT_DOCUMENT_ID);
        row.add(Root.COLUMN_AVAILABLE_BYTES, 1024L * 1024L);
        return cursor;
    }

    @Override
    public Cursor queryDocument(String documentId, String[] projection)
        throws FileNotFoundException {
        MatrixCursor cursor = new MatrixCursor(resolveDocumentProjection(projection));
        includeDocument(cursor, requireDocument(documentId));
        return cursor;
    }

    @Override
    public Cursor queryChildDocuments(
        String parentDocumentId,
        String[] projection,
        String sortOrder
    ) throws FileNotFoundException {
        requireDocument(parentDocumentId);
        MatrixCursor cursor = new MatrixCursor(resolveDocumentProjection(projection));
        for (QaDocument document : documents.values()) {
            if (parentDocumentId.equals(document.parentId)) {
                includeDocument(cursor, document);
            }
        }
        return cursor;
    }

    @Override
    public ParcelFileDescriptor openDocument(
        String documentId,
        String mode,
        CancellationSignal signal
    ) throws FileNotFoundException {
        if (!"r".equals(mode)) {
            throw new FileNotFoundException("QA provider is read-only");
        }
        QaDocument document = requireDocument(documentId);
        if (document.assetPath == null) {
            throw new FileNotFoundException("Cannot open a directory");
        }
        try {
            ParcelFileDescriptor[] pipe = ParcelFileDescriptor.createPipe();
            Thread writer = new Thread(
                () -> copyAssetToPipe(document.assetPath, pipe[1]),
                "hiddenshield-qa-provider"
            );
            writer.start();
            return pipe[0];
        } catch (IOException error) {
            throw new FileNotFoundException(error.getMessage());
        }
    }

    @Override
    public String getDocumentType(String documentId) throws FileNotFoundException {
        return requireDocument(documentId).mimeType;
    }

    @Override
    public boolean isChildDocument(String parentDocumentId, String documentId) {
        QaDocument current = documents.get(documentId);
        while (current != null && current.parentId != null) {
            if (parentDocumentId.equals(current.parentId)) {
                return true;
            }
            current = documents.get(current.parentId);
        }
        return false;
    }

    private void copyAssetToPipe(String assetPath, ParcelFileDescriptor writeSide) {
        try (
            InputStream input = getContext().getAssets().open(assetPath);
            OutputStream output = new ParcelFileDescriptor.AutoCloseOutputStream(writeSide)
        ) {
            byte[] buffer = new byte[8192];
            int read;
            while ((read = input.read(buffer)) != -1) {
                output.write(buffer, 0, read);
            }
        } catch (IOException ignored) {
        }
    }

    private void includeDocument(MatrixCursor cursor, QaDocument document) {
        MatrixCursor.RowBuilder row = cursor.newRow();
        row.add(Document.COLUMN_DOCUMENT_ID, document.id);
        row.add(Document.COLUMN_MIME_TYPE, document.mimeType);
        row.add(Document.COLUMN_DISPLAY_NAME, document.displayName);
        row.add(Document.COLUMN_LAST_MODIFIED, 0L);
        row.add(
            Document.COLUMN_FLAGS,
            Document.MIME_TYPE_DIR.equals(document.mimeType)
                ? Document.FLAG_DIR_PREFERS_GRID
                : 0
        );
        row.add(Document.COLUMN_SIZE, null);
    }

    private QaDocument requireDocument(String documentId)
        throws FileNotFoundException {
        QaDocument document = documents.get(documentId);
        if (document == null) {
            throw new FileNotFoundException("Unknown document: " + documentId);
        }
        return document;
    }

    private void addDirectory(String id, String parentId, String displayName) {
        documents.put(
            id,
            new QaDocument(
                id,
                parentId,
                displayName,
                Document.MIME_TYPE_DIR,
                null
            )
        );
    }

    private void addFile(
        String id,
        String parentId,
        String displayName,
        String mimeType,
        String assetPath
    ) {
        documents.put(
            id,
            new QaDocument(id, parentId, displayName, mimeType, assetPath)
        );
    }

    private static String[] resolveRootProjection(String[] projection) {
        return projection == null ? ROOT_COLUMNS : projection;
    }

    private static String[] resolveDocumentProjection(String[] projection) {
        return projection == null ? DOCUMENT_COLUMNS : projection;
    }

    private static final class QaDocument {
        final String id;
        final String parentId;
        final String displayName;
        final String mimeType;
        final String assetPath;

        QaDocument(
            String id,
            String parentId,
            String displayName,
            String mimeType,
            String assetPath
        ) {
            this.id = id;
            this.parentId = parentId;
            this.displayName = displayName;
            this.mimeType = mimeType;
            this.assetPath = assetPath;
        }
    }
}
