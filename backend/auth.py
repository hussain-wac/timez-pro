import os
import json
import logging
from datetime import datetime, timedelta, timezone
from typing import Optional

from fastapi import Depends, HTTPException, status
from fastapi.security import HTTPBearer, HTTPAuthorizationCredentials
from jose import JWTError, jwt
from google.oauth2 import id_token
from google.auth.transport import requests
from sqlalchemy.orm import Session

from database import get_db
from models import User

logger = logging.getLogger(__name__)

# Configuration - set these as environment variables
SECRET_KEY = os.getenv("SECRET_KEY", "your-secret-key-change-in-production")
ALGORITHM = "HS256"
ACCESS_TOKEN_EXPIRE_DAYS = 30
GOOGLE_CLIENT_ID = os.getenv("GOOGLE_CLIENT_ID", "")
GOOGLE_CLIENT_SECRET = os.getenv("GOOGLE_CLIENT_SECRET", "")

# Parse client IDs - support both comma-separated and JSON array formats
client_ids_str = os.getenv("GOOGLE_CLIENT_IDS", GOOGLE_CLIENT_ID)
try:
    # Try parsing as JSON array first
    GOOGLE_CLIENT_IDS = json.loads(client_ids_str) if client_ids_str.startswith('[') else [c.strip() for c in client_ids_str.split(',') if c.strip()]
except:
    GOOGLE_CLIENT_IDS = [c.strip() for c in client_ids_str.split(',') if c.strip()]

ADMIN_EMAILS = os.getenv("ADMIN_EMAILS", "hussain.n@webandcrafts.in").split(",")

security = HTTPBearer()


def verify_google_token(token: str) -> dict:
    """Verify Google ID token and return user info."""
    # Try each client ID - try the one that matches token audience first
    for client_id in GOOGLE_CLIENT_IDS:
        if not client_id:
            continue
        try:
            logger.info(f"Trying Google token verification with client ID: {client_id}")
            idinfo = id_token.verify_oauth2_token(
                token, requests.Request(), client_id
            )
            logger.info(f"Token verified successfully for: {idinfo.get('email')} with client ID: {client_id}")

            if idinfo["iss"] not in ["accounts.google.com", "https://accounts.google.com"]:
                raise ValueError("Invalid issuer")

            return {
                "google_id": idinfo["sub"],
                "email": idinfo["email"],
                "name": idinfo.get("name"),
                "picture": idinfo.get("picture"),
            }
        except Exception as e:
            error_msg = str(e)
            # If it's audience mismatch, try next client ID
            if "wrong audience" in error_msg.lower():
                logger.warning(f"Token audience mismatch for {client_id}, trying next...")
                continue
            # If it's HS256 error, it's a token format issue, skip
            if "HS256" in error_msg:
                logger.warning(f"HS256 error for {client_id}, skipping...")
                continue
            logger.warning(f"Failed with client ID {client_id}: {error_msg}")
            continue
    
    logger.error("All Google client IDs failed")
    raise HTTPException(
        status_code=status.HTTP_401_UNAUTHORIZED,
        detail="Invalid Google token",
    )


def create_access_token(user_id: int) -> str:
    """Create JWT access token."""
    expire = datetime.now(timezone.utc) + timedelta(days=ACCESS_TOKEN_EXPIRE_DAYS)
    to_encode = {"sub": str(user_id), "exp": expire}
    return jwt.encode(to_encode, SECRET_KEY, algorithm=ALGORITHM)


def get_user_from_token(token: str, db: Session) -> User:
    """Decode JWT token and return user."""
    credentials_exception = HTTPException(
        status_code=status.HTTP_401_UNAUTHORIZED,
        detail="Could not validate credentials",
        headers={"WWW-Authenticate": "Bearer"},
    )
    try:
        payload = jwt.decode(token, SECRET_KEY, algorithms=[ALGORITHM])
        user_id: str = payload.get("sub")
        if user_id is None:
            raise credentials_exception
    except JWTError:
        raise credentials_exception

    user = db.query(User).filter(User.id == int(user_id)).first()
    if user is None:
        raise credentials_exception
    return user


def get_current_user(
    credentials: HTTPAuthorizationCredentials = Depends(security),
    db: Session = Depends(get_db),
) -> User:
    """Dependency to get the current authenticated user."""
    return get_user_from_token(credentials.credentials, db)


def get_or_create_user(db: Session, google_user: dict) -> User:
    """Get existing user or create new one from Google user info."""
    # First try to find by google_id
    user = db.query(User).filter(User.google_id == google_user["google_id"]).first()

    if not user:
        # Fallback: find by email (handles placeholder accounts)
        user = db.query(User).filter(User.email == google_user["email"]).first()

    is_admin = google_user["email"] in ADMIN_EMAILS

    if user:
        # Update user info (including google_id for placeholder accounts)
        user.google_id = google_user["google_id"]
        user.name = google_user["name"]
        user.picture = google_user["picture"]
        user.is_admin = is_admin
        db.commit()
        db.refresh(user)
    else:
        # Create new user
        user = User(
            email=google_user["email"],
            name=google_user["name"],
            picture=google_user["picture"],
            google_id=google_user["google_id"],
            is_admin=is_admin,
        )
        db.add(user)
        db.commit()
        db.refresh(user)

    return user
