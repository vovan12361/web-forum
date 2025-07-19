#!/usr/bin/env python3
import os
import json
import logging
from fastapi import FastAPI, Request
import uvicorn
import telegram
from telegram.ext import Updater

# Configure logging
logging.basicConfig(
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    level=logging.INFO
)
logger = logging.getLogger(__name__)

# Get environment variables
TELEGRAM_BOT_TOKEN = os.environ.get("TELEGRAM_BOT_TOKEN")
TELEGRAM_CHAT_ID = os.environ.get("TELEGRAM_CHAT_ID")

if not TELEGRAM_BOT_TOKEN or not TELEGRAM_CHAT_ID:
    logger.error("TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID environment variables must be set")
    exit(1)

# Initialize bot
bot = telegram.Bot(token=TELEGRAM_BOT_TOKEN)
app = FastAPI()

@app.get("/health")
async def health_check():
    return {"status": "ok"}

@app.post("/alert")
async def alert_handler(request: Request):
    data = await request.json()
    logger.info(f"Received alert: {data}")
    
    try:
        alerts = data.get('alerts', [])
        
        if not alerts:
            return {"status": "ok", "message": "No alerts in the payload"}
        
        for alert in alerts:
            status = alert.get('status', 'unknown')
            labels = alert.get('labels', {})
            annotations = alert.get('annotations', {})
            
            alert_name = labels.get('alertname', 'Unknown Alert')
            summary = annotations.get('summary', 'No summary provided')
            description = annotations.get('description', 'No description provided')
            
            emoji = "🔴" if status == "firing" else "✅"
            
            message = f"{emoji} *{alert_name}* - {status.upper()}\n\n"
            message += f"*Summary:* {summary}\n"
            message += f"*Description:* {description}\n"
            
            if status == "firing":
                message += "\n*Please check the system immediately!*"
            else:
                message += "\n*Alert resolved. No further action needed.*"
            
            await bot.send_message(
                chat_id=TELEGRAM_CHAT_ID,
                text=message,
                parse_mode=telegram.ParseMode.MARKDOWN
            )
        
        return {"status": "ok", "message": f"Sent {len(alerts)} alerts to Telegram"}
    
    except Exception as e:
        logger.error(f"Error processing alert: {str(e)}")
        return {"status": "error", "message": str(e)}

# Startup and shutdown events
@app.on_event("startup")
async def startup_event():
    logger.info("Starting Alert Bot...")
    await bot.send_message(
        chat_id=TELEGRAM_CHAT_ID,
        text="🚀 *Alert Bot is now active*\nMonitoring system alerts.",
        parse_mode=telegram.ParseMode.MARKDOWN
    )

@app.on_event("shutdown")
async def shutdown_event():
    logger.info("Shutting down Alert Bot...")
    try:
        await bot.send_message(
            chat_id=TELEGRAM_CHAT_ID,
            text="⚠️ *Alert Bot is shutting down*\nAlert notifications will be unavailable.",
            parse_mode=telegram.ParseMode.MARKDOWN
        )
    except:
        pass

if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=8080) 