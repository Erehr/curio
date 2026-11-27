"""Notes each item's size in its notes. Curio gives the items on stdin and, in the environment, CURIO_API_URL and CURIO_API_TOKEN for this run."""
import json
import os
import sys
import urllib.request

API = os.environ["CURIO_API_URL"]
TOKEN = os.environ["CURIO_API_TOKEN"]


def call(op, params):
    req = urllib.request.Request(API + op.replace(".", "/"), json.dumps(params).encode(), {"Authorization": f"Bearer {TOKEN}", "Content-Type": "application/json"})
    with urllib.request.urlopen(req) as r:
        return json.load(r)


given = json.load(sys.stdin)
ids = [a["uid"] for a in given["assets"]]
for it in call("item.getMany", {"ids": ids}):
    size = f"{it['size'] / 1048576:.1f} MB"
    note = (it.get("annotation") or "").strip()
    if size not in note:
        call("item.update", {"id": it["id"], "annotation": f"{note}\n{size}".strip()})
json.dump({"message": f"Noted the size of {len(ids)} item(s)"}, sys.stdout)
