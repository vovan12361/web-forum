FROM python:3.10-slim

WORKDIR /mnt/locust

RUN pip install --no-cache-dir locust

EXPOSE 8089

ENTRYPOINT ["locust", "--host", "${TARGET_HOST}", "--web-host", "0.0.0.0"] 