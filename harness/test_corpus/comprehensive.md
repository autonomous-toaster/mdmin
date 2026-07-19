# Comprehensive Test Document

> This blockquote introduces the document and explains its purpose.
> It contains multiple lines to test blockquote preservation.
> Blockquotes are important for LLM context understanding.

## Section 1: Code Examples

Here is some inline code: `print("hello world")` and `const x = 42;`.

```python
def hello():
    print("Hello, World!")
    return 42

def goodbye():
    print("Goodbye!")
```

```javascript
function greet(name) {
    console.log(`Hello, ${name}!`);
    return true;
}
```

> Code blocks should preserve their content and language annotations.
> The LLM needs to understand which language each block uses.

## Section 2: Tables

| Name | Age | City | Occupation |
|------|-----|------|------------|
| Alice | 30 | New York | Engineer |
| Bob | 25 | London | Designer |
| Charlie | 35 | Tokyo | Manager |

| Product | Price | Stock |
|---------|-------|-------|
| Widget A | $10.99 | 100 |
| Widget B | $24.99 | 50 |
| Widget C | $5.99 | 200 |

> Tables contain structured data that must be preserved for LLM comprehension.
> Cell values like "Alice", "Engineer", and "$10.99" should survive compression.

## Section 3: Links and References

Here are some useful resources:

- [OpenAI](https://openai.com) - AI research and deployment
- [GitHub](https://github.com) - Code hosting platform
- [Python Documentation](https://docs.python.org/3/) - Official Python docs
- [ArXiv](https://arxiv.org) - Research papers repository

> Links should have their URLs preserved even after protocol stripping.
> The LLM needs the actual URLs to access resources.

## Section 4: Lists

Shopping list:
- Apples
- Bananas
- Cherries
- Dates
- Elderberries

Task list:
* Write documentation
* Run tests
* Deploy to production
* Monitor performance
* Review logs

Nested list:
- Fruits
  - Tropical
    - Mango
    - Papaya
  - Citrus
    - Orange
    - Lemon
- Vegetables
  - Leafy greens
    - Spinach
    - Kale

> Lists should preserve their items and hierarchy.
> Nested lists are important for structured information.

## Section 5: Inline Code and Formatting

Use the `os.path.join()` function to combine paths.
The `subprocess.run()` function executes commands.
Call `api.fetch_data(user_id, limit=10)` to get results.

Key variables: `API_KEY`, `DATABASE_URL`, `MAX_RETRIES`.

> Inline code spans contain important identifiers and commands.
> The LLM needs these to understand technical details.

## Section 6: Edge Cases

Empty code block:

```

```

Single character: `a`

Minimal heading: # X

Table with empty cells:

| Left | Center | Right |
|------|--------|-------|
| A | | C |
| | B | |

> Edge cases test the robustness of the compression.
> Empty cells and minimal content should be handled gracefully.
