from setuptools import setup, find_packages

setup(
    name="jsonschema",
    version="0.0.0-local",
    description="Minimal offline jsonschema implementation",
    packages=find_packages(),
    package_dir={'': '.'},
    python_requires=">=3.8",
)
