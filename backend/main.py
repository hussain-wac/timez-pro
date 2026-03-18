from dotenv import load_dotenv
load_dotenv()

import os
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from database import engine, Base
from routes import tasks, timer, reports, auth, dashboard

# Create database tables
Base.metadata.create_all(bind=engine)

app = FastAPI(
    title="Time Tracker API",
    description="A time-tracking application API similar to Hubstaff",
    version="1.0.0",
)

# CORS configuration
CORS_ORIGINS = os.getenv("CORS_ORIGINS", "http://localhost:5173,http://localhost:3000").split(",")
app.add_middleware(
    CORSMiddleware,
    allow_origins=CORS_ORIGINS,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Include routers
app.include_router(auth.router)
app.include_router(tasks.router)
app.include_router(timer.router)
app.include_router(reports.router)
app.include_router(dashboard.router)


@app.get("/")
def root():
    return {"message": "Time Tracker API", "docs": "/docs"}
