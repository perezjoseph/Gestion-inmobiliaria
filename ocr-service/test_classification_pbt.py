"""Property-based tests for _classify_document in the OCR sidecar."""

from hypothesis import given, settings
from hypothesis import strategies as st

from main import _classify_document


CEDULA_KEYWORDS = ["CEDULA", "IDENTIDAD", "ELECTORAL", "REPUBLICA DOMINICANA"]
CONTRATO_KEYWORDS = ["CONTRATO", "ARRENDAMIENTO", "ALQUILER"]
DEPOSITO_KEYWORDS = ["DEPOSITO", "AHORROS", "BANCO"]
GASTO_KEYWORDS = ["FACTURA", "RECIBO", "COMPROBANTE"]

ALL_NON_CEDULA_KEYWORDS = CONTRATO_KEYWORDS + DEPOSITO_KEYWORDS + GASTO_KEYWORDS

EXPENSE_KEYWORDS = GASTO_KEYWORDS


def _lines_from_texts(texts: list[str]) -> list[dict]:
    return [{"text": t, "confidence": 0.9, "bbox": [0, 0, 100, 0, 100, 30, 0, 30]} for t in texts]


filler_text = st.text(
    alphabet=st.characters(whitelist_categories=("L", "N", "Zs"), min_codepoint=32, max_codepoint=122),
    min_size=0,
    max_size=30,
)


@given(
    cedula_kw=st.sampled_from(CEDULA_KEYWORDS),
    other_kws=st.lists(st.sampled_from(ALL_NON_CEDULA_KEYWORDS), min_size=0, max_size=4),
    extra_lines=st.lists(filler_text, min_size=0, max_size=5),
)
@settings(max_examples=200)
def test_cedula_classification_takes_priority(cedula_kw, other_kws, extra_lines):
    """Property 2: Cédula classification takes priority.

    For any set of OCR lines where the combined text contains at least one
    cédula keyword, _classify_document returns "cedula" regardless of whether
    deposit, expense, or contract keywords are also present.

    **Validates: Requirements 2.1, 2.5**
    """
    texts = [cedula_kw] + other_kws + extra_lines
    lines = _lines_from_texts(texts)
    assert _classify_document(lines) == "cedula"


@given(
    contrato_kw=st.sampled_from(CONTRATO_KEYWORDS),
    expense_kws=st.lists(st.sampled_from(EXPENSE_KEYWORDS), min_size=1, max_size=4),
    extra_lines=st.lists(filler_text, min_size=0, max_size=5),
)
@settings(max_examples=200)
def test_contrato_classification_takes_priority_over_expense(contrato_kw, expense_kws, extra_lines):
    """Property 3: Contract classification takes priority over expense.

    For any set of OCR lines where the combined text contains at least one
    contract keyword plus expense keywords (and no cédula keywords),
    _classify_document returns "contrato".

    **Validates: Requirements 3.1, 3.6**
    """
    texts = [contrato_kw] + expense_kws + extra_lines
    lines = _lines_from_texts(texts)
    assert _classify_document(lines) == "contrato"
