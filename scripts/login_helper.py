#!/usr/bin/env python3
"""Helper script to login to Amazon and output cookies for alexa-cli."""
import asyncio
import json
import sys

from aioamazondevices.login import AmazonLogin
from aioamazondevices.http_wrapper import AmazonHttpWrapper, AmazonSessionStateData
from aiohttp import ClientSession


async def do_login(email: str, password: str, otp: str = "") -> dict:
    login_site = "https://www.amazon.com"
    
    session_state = AmazonSessionStateData(
        login_site=login_site,
        login_email=email,
        login_password=password,
    )
    
    async with ClientSession() as session:
        http_wrapper = AmazonHttpWrapper(session, session_state)
        amazon_login = AmazonLogin(http_wrapper, session_state)
        
        login_data = await amazon_login.login_mode_interactive(otp)
        return login_data


def main():
    if len(sys.argv) < 3:
        print("Usage: login_helper.py <email> <password> [otp]", file=sys.stderr)
        sys.exit(1)
    
    email = sys.argv[1]
    password = sys.argv[2]
    otp = sys.argv[3] if len(sys.argv) > 3 else ""
    
    # If no OTP provided, first attempt triggers Amazon to send the code
    if not otp:
        # Try with empty OTP - this will fail but triggers the SMS/push
        try:
            login_data = asyncio.run(do_login(email, password, "000000"))
            print(json.dumps(login_data.get("website_cookies", {})))
            return
        except Exception:
            # Expected to fail - OTP was sent to phone
            print("OTP_NEEDED", flush=True)
            sys.exit(2)
    
    try:
        login_data = asyncio.run(do_login(email, password, otp))
        print(json.dumps(login_data.get("website_cookies", {})))
    except Exception as e:
        print(f"Login failed: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
