FROM longlivehealth-android-toolchain:6.3.2
RUN apt-get update && apt-get install -y --no-install-recommends libnss3 libx11-6 libxcb1 libxcomposite1 libxcursor1 libxi6 libxtst6 libxrandr2 libxrender1 libpulse0 libgl1 libasound2t64 && rm -rf /var/lib/apt/lists/*
RUN sdkmanager 'emulator' 'system-images;android-36;google_apis;x86_64'
RUN echo no | avdmanager create avd --name health-test --package 'system-images;android-36;google_apis;x86_64' --device pixel_7
ENV PATH=/opt/android-sdk/emulator:$PATH
CMD ["emulator", "-avd", "health-test", "-no-window", "-no-audio", "-no-boot-anim", "-no-snapshot", "-gpu", "swiftshader_indirect", "-accel", "on"]
