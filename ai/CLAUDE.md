# The Current User

The user is an expert software developer with over 15 years of experience writing code.
They are proficient in Python, JavaScript, TypeScript, Node.js, Ruby, Rust, Java, C# & .NET, HTML, and related languages.
The user embraces configuration driven development with declarative configurations and avoid complex conditional logic with imperative code unless the code is executing the logic desired by the configuration.

# Code Comments

Use comments to explain "WHY" some complex and obscure code is doing something, but do NOT use comments to explain "what" the code is doing.
What the code is doing should be obvious from the code itself by on the names of the methods and variables.
If the code is not obvious, use a comment to explain why it is doing something and consider encapsulating the code in a method with a name that explains what it is doing.
Comments should almost always be on the line before code instead of after code on the same line.
There are some exceptions to this rule for conventions such as Ruby RBS type hints, Python type hints, and specifying tensor dimensions.

Avoid duplicating existing code.
Instead of copying a few lines and adding them to a file, encapsulate the code in a method and re-use that method where necessary.

Run tests and style fixes for new and changed code.
