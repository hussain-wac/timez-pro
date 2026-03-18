from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session

from database import get_db
from schemas import GoogleAuthRequest, AuthResponse, UserResponse
from auth import (
    verify_google_token,
    create_access_token,
    get_or_create_user,
    get_current_user,
)
from models import User

router = APIRouter(prefix="/api/auth", tags=["auth"])


@router.post("/google", response_model=AuthResponse)
def google_login(request: GoogleAuthRequest, db: Session = Depends(get_db)):
    """Authenticate with Google ID token."""
    # Verify the Google token
    google_user = verify_google_token(request.token)

    # Get or create user
    user = get_or_create_user(db, google_user)

    # Create access token
    access_token = create_access_token(user.id)

    return AuthResponse(
        access_token=access_token,
        user=UserResponse(
            id=user.id,
            email=user.email,
            name=user.name,
            picture=user.picture,
            is_admin=user.is_admin,
            created_at=user.created_at,
        ),
    )


@router.get("/me", response_model=UserResponse)
def get_me(current_user: User = Depends(get_current_user)):
    """Get current authenticated user."""
    return UserResponse(
        id=current_user.id,
        email=current_user.email,
        name=current_user.name,
        picture=current_user.picture,
        is_admin=current_user.is_admin,
        created_at=current_user.created_at,
    )
