FROM python:3.10-slim

WORKDIR /mnt/locust

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

EXPOSE 8089

ENTRYPOINT ["locust", "--host", "${TARGET_HOST}", "--web-host", "0.0.0.0"] 