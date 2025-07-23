#!/usr/bin/env python3
"""
String to Hex Converter Utility

This script converts strings to their hex representation, similar to the conversion:
print(f'{"USDC.axl".encode().hex().upper():040}')

Usage:
    python string_to_hex_converter.py "USDC.axl"
    python string_to_hex_converter.py "USDC.axl" --padding 40
    python string_to_hex_converter.py "USDC.axl" --no-uppercase
"""

import argparse
import sys


def string_to_hex(input_string, padding=40, uppercase=True):
    """
    Convert a string to its hex representation with optional padding and case conversion.
    
    Args:
        input_string (str): The string to convert
        padding (int): Number of characters to pad to (default: 40)
        uppercase (bool): Whether to convert to uppercase (default: True)
    
    Returns:
        str: The hex representation with padding
    """
    # Convert string to bytes, then to hex
    hex_string = input_string.encode().hex()
    
    # Convert to uppercase if requested
    if uppercase:
        hex_string = hex_string.upper()
    
    # Pad with zeros after the hex string to the specified length
    padded_hex = f"{hex_string}{'0' * (padding - len(hex_string))}"
    
    return padded_hex


def main():
    parser = argparse.ArgumentParser(
        description="Convert strings to hex representation with padding",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s "USDC.axl"
  %(prog)s "USDC.axl" --padding 40
  %(prog)s "USDC.axl" --no-uppercase
  %(prog)s "Hello World" --padding 32
        """
    )
    
    parser.add_argument(
        "string",
        help="The string to convert to hex"
    )
    
    parser.add_argument(
        "--padding",
        type=int,
        default=40,
        help="Number of characters to pad to (default: 40)"
    )
    
    parser.add_argument(
        "--no-uppercase",
        action="store_true",
        help="Keep hex in lowercase (default: uppercase)"
    )
    
    parser.add_argument(
        "--format",
        choices=["simple", "detailed", "python"],
        default="simple",
        help="Output format (default: simple)"
    )
    
    args = parser.parse_args()
    
    try:
        result = string_to_hex(args.string, args.padding, not args.no_uppercase)
        
        if args.format == "simple":
            print(result)
        elif args.format == "detailed":
            print(f"Input: {args.string}")
            print(f"Hex: {result}")
            print(f"Length: {len(result)}")
        elif args.format == "python":
            print(f'print(f\'{{"{args.string}".encode().hex().upper():0{args.padding}}}\')')
            print(f"# Result: {result}")
            
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
