-- Person onboarding channel + preferred contact method
-- Tracks how a person first engaged with PCG and their preferred communication channel

ALTER TABLE persons ADD COLUMN onboarding_channel TEXT
    CHECK(onboarding_channel IN ('email','instagram','whatsapp','linkedin','twitter','sms','phone','in_person'));

ALTER TABLE persons ADD COLUMN preferred_contact TEXT
    CHECK(preferred_contact IN ('email','instagram','whatsapp','linkedin','twitter','sms','phone','in_person'));
