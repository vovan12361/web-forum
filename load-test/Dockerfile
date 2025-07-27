FROM python:3.10-slim

WORKDIR /mnt/locust

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

EXPOSE 8089

ENTRYPOINT ["locust", "--host", "http://app:8080", "--web-host", "0.0.0.0"] 