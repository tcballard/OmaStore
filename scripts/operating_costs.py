"""Calculate a single-currency operating scenario without inventing provider quotes."""
import json
from decimal import Decimal, ROUND_HALF_UP
from pathlib import Path
import sys

p = json.loads(Path(sys.argv[1]).read_text())
names = ["monthlyInfrastructureMinor", "monthlyStorageAndEgressMinor", "monthlyMonitoringBackupMinor", "newSubmissionsPerMonth", "updatesPerMonth", "minutesPerNewReview", "minutesPerUpdateReview", "operatorHourlyMinor", "monthlySupportHours"]
missing = [k for k in names if p.get(k) is None]
if missing:
    print(json.dumps({"complete": False, "missingInputs": missing}, indent=2))
    sys.exit(2)
if any(type(p[k]) not in (int, float) or not 0 <= p[k] <= 10**12 for k in names):
    raise ValueError("Inputs must be finite nonnegative bounded numbers")
if not isinstance(p.get("currency"), str) or len(p["currency"]) != 3 or p.get("minorUnitExponent") not in (0, 2, 3):
    raise ValueError("Specify a currency and its minor-unit exponent")
v = {k: Decimal(str(p[k])) for k in names}
hours = (v["newSubmissionsPerMonth"] * v["minutesPerNewReview"] + v["updatesPerMonth"] * v["minutesPerUpdateReview"]) / 60
labor = ((hours + v["monthlySupportHours"]) * v["operatorHourlyMinor"]).quantize(Decimal(1), rounding=ROUND_HALF_UP)
infra = sum(v[k] for k in names[:3])
print(json.dumps({"complete": True, "currency": p["currency"], "reviewHoursPerMonth": str(hours), "laborMinor": str(labor), "infrastructureMinor": str(infra), "monthlyTotalMinor": str(labor + infra), "notice": "Scenario from supplied inputs; provider fees, tax and real checkout revenue are not assumed."}, indent=2))
