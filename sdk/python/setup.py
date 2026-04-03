from setuptools import setup

setup(
    name="rustyhand",
    version="0.1.0",
    description="Official Python client for the RustyHand Agent OS REST API",
    py_modules=["rustyhand_sdk", "rustyhand_client"],
    python_requires=">=3.8",
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
    ],
)
