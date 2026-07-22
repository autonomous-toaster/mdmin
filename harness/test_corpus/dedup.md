# Semantic Deduplication Test

## Section 1: Repeated Code Patterns

The following import pattern appears multiple times:

```python
from typing import Optional
from typing import List
from typing import Dict
```

Later in the document, the same import pattern appears again:

```python
from typing import Optional
from typing import List
from typing import Dict
```

And again in a different context:

```python
from typing import Optional
from typing import List
from typing import Dict
```

## Section 2: Repeated Prose Patterns

This is a long and verbose description that appears multiple times in the document. It contains important information that should be preserved but doesn't need to be repeated verbatim.

This is a long and verbose description that appears multiple times in the document. It contains important information that should be preserved but doesn't need to be repeated verbatim.

## Section 3: Repeated Configuration Blocks

```yaml
server:
  host: localhost
  port: 8080
  timeout: 30
  retries: 3
```

The same configuration appears in another section:

```yaml
server:
  host: localhost
  port: 8080
  timeout: 30
  retries: 3
```

## Section 4: Repeated URLs and Paths

Reference: https://docs.example.com/api/v1/users
Reference: https://docs.example.com/api/v1/users
Reference: https://docs.example.com/api/v1/users

## Section 5: Unique Content (should not be affected)

This section contains unique content that appears only once. It should be preserved as-is.

```python
def unique_function():
    return "this is unique"
```

The quick brown fox jumps over the lazy dog. This sentence is unique and should not be duplicated.
