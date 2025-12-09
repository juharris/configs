# The Current User

The user is an expert software developer with over 15 years of experience writing code.
They are proficient in Python, JavaScript, TypeScript, Node.js, Ruby, Rust, Java, C# & .NET, HTML, and related languages.
The user embraces configuration driven development with declarative configurations and avoid complex conditional logic with imperative code unless the code is executing the logic desired by the configuration.

# Code Style

Prefer encapsulation and re-use over duplication such as copying existing code.
Instead of copying code, encapsulate the code in a method or class and re-use that method or class where necessary.
Executives such as the CEO and CTO of the large multi-national corporation where the user works reviews all of their code so the code needs to be robust and clear.

# Code Comments

Use comments to explain "WHY" some complex and obscure code is doing something, but do NOT use comments to explain "what" the code is doing.
What the code is doing should be obvious from the code itself by on the names of the methods and variables.
If the code is not obvious, use a comment to explain why it is doing something and consider encapsulating the code in a method with a name that explains what it is doing.
Comments should almost always be on the line before code instead of after code on the same line.
There are some exceptions to this rule for conventions such as Ruby RBS type hints, Python type hints, and specifying tensor dimensions.

Do not delete existing comments when making changes, unless the comments are not true anymore.
There are often existing comments that explain why the code is doing something and those comments should be preserved, unless they are no longer true.

# After Making Changes

Run tests and style fixes for new and changed code.
