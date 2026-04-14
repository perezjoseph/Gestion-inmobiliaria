from fastapi import FastAPI, File, UploadFile
from fastapi.responses import JSONResponse

app = FastAPI(title="OCR Service")


@app.get("/health")
async def health():
    return {"status": "ok"}


@app.post("/ocr/extract")
async def ocr_extract(image: UploadFile = File(...)):
    file_bytes = await image.read()
    return JSONResponse(content={
        "document_type": "unknown",
        "lines": [],
        "structured_fields": {},
    })
